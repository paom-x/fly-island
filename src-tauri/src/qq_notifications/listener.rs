//! WinRT user notification listener: reads QQ toasts and repairs the QQ registration.

use base64::{engine::general_purpose::STANDARD, Engine};
use std::{
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};
use windows::{
    core::{HRESULT, HSTRING},
    Foundation::TypedEventHandler,
    UI::Notifications::{
        KnownNotificationBindings,
        Management::{UserNotificationListener, UserNotificationListenerAccessStatus},
        NotificationKinds, NotificationSetting, ToastNotificationManager, UserNotification,
        UserNotificationChangedEventArgs,
    },
};

use super::history::NotificationBuffer;
use super::{publish, NotificationService, NotificationStatus, QQNotification};
use crate::win32;

const QQ_APP_ID: &str = "QQ";

const REGISTRATION_REPAIR: &str =
    include_str!("repair-qq-notification-registration.ps1");

const DIAGNOSTIC_TITLE: &str = "Fly Island QQ 通知诊断";

const DIAGNOSTIC_BODY: &str = "这是本机通道测试，不是聊天消息。";

/// How often QQ's own notification setting is looked at while it is fine, and while it is not.
const SETTING_CHECK: Duration = Duration::from_secs(30);
const SETTING_RECHECK_WHILE_BROKEN: Duration = Duration::from_secs(5);

/// How long the listener may sit still before it re-reads the list on its own. Reads are woken by
/// the system's own change event, so this only catches the cases where no event ever arrives (a
/// listener that was never granted access, a channel that was lost).
pub(super) const SAFETY_POLL: Duration = Duration::from_secs(30);

/// The read interval when no change event could be subscribed: a notification has to show up within
/// a second, so the store is read once a second, as the listener did before the event existed.
pub(super) const POLL_FALLBACK: Duration = Duration::from_secs(1);

/// One live notification listener, kept open for the life of the monitor thread.
///
/// Opening it per read (as this used to) entered and left the Windows Runtime every second and
/// asked the notification store for its whole list on the same beat: about 2.7% of a core on an
/// idle machine, for a list that changes a few times an hour. The listener now stays open and
/// the system's own change event wakes the loop; the list is re-read only when it says something
/// happened.
struct Channel {
    listener: UserNotificationListener,
    /// The change-event subscription, when the system granted one. An unpackaged app is refused
    /// here (`ERROR_NOT_FOUND`, 0x80070490) even though the listener itself opens fine, so the
    /// token and its handler live in an `Option` and the monitor polls when they are absent.
    subscription: Option<(
        i64,
        TypedEventHandler<UserNotificationListener, UserNotificationChangedEventArgs>,
    )>,
}

impl Channel {
    /// Opens the listener and asks `wake` to nudge the monitor thread whenever Windows reports a
    /// notification change.
    fn open(wake: &mpsc::SyncSender<()>) -> windows::core::Result<Self> {
        let listener = UserNotificationListener::Current()?;
        let sink = wake.clone();
        // A full channel means a wake-up is already waiting, which is all an event has to say.
        let handler = TypedEventHandler::new(move |_, _| {
            let _ = sink.try_send(());
            Ok(())
        });
        // Losing the subscription is not losing the listener: without it the monitor reads on a
        // short poll instead, which is exactly what it did before the event existed.
        let subscription = match listener.NotificationChanged(&handler) {
            Ok(token) => Some((token, handler)),
            Err(error) => {
                eprintln!("QQ 通知变化事件订阅失败，改为每秒读取：{error}");
                None
            }
        };
        Ok(Self {
            listener,
            subscription,
        })
    }

    /// Whether Windows pushes changes to the monitor, or it has to look for them itself.
    fn wakes_on_events(&self) -> bool {
        self.subscription.is_some()
    }

    fn read(&self) -> Result<(NotificationStatus, Vec<QQNotification>), String> {
        read_from(&self.listener)
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        if let Some((token, _)) = &self.subscription {
            let _ = self.listener.RemoveNotificationChanged(*token);
        }
    }
}

pub(super) fn monitor(service: Arc<NotificationService>, receiver: mpsc::Receiver<()>) {
    // Started at sign-in: wait for the desktop to settle first (the listener may repair QQ's
    // registration through PowerShell).
    crate::system::startup::hold_after_autostart(std::time::Duration::from_secs(30));
    // Held for the life of the thread: the listener is created and dropped inside it, and the
    // Windows Runtime would be torn down and set up again around every read otherwise.
    let _apartment = win32::com::WinRtApartment::multithreaded().ok();
    let mut channel: Option<Channel> = None;
    let mut repair_generation = None;
    let mut repair_error = None;
    // When QQ's setting was last looked at, for which generation, and what was wrong then.
    let mut channel_check: Option<(u64, Instant, Option<String>)> = None;
    loop {
        let (enabled, generation) = match service.state.lock() {
            Ok(state) => (state.snapshot.enabled, state.generation),
            Err(_) => return,
        };
        if !enabled {
            if receiver.recv().is_err() {
                return;
            }
            continue;
        }
        if channel.is_none() {
            channel = Channel::open(&service.wake).ok();
        }
        let poll = match channel.as_ref() {
            Some(open) => open.read(),
            // Nothing to read from: the status names it, and the safety poll tries again.
            None => Err("Windows 通知监听不可用，正在重试。".to_string()),
        };
        if poll.is_err() {
            // A lost listener is rebuilt from scratch on the next round.
            channel = None;
        }
        let channel_error = if matches!(&poll, Ok((NotificationStatus::Listening, _))) {
            let due = channel_check.as_ref().is_none_or(|(checked_for, at, error)| {
                *checked_for != generation
                    || at.elapsed() >= if error.is_some() { SETTING_RECHECK_WHILE_BROKEN } else { SETTING_CHECK }
            });
            if due {
                let error = check_channel(&service, generation, &mut repair_generation, &mut repair_error);
                channel_check = Some((generation, Instant::now(), error.clone()));
                error
            } else {
                channel_check.as_ref().and_then(|(_, _, error)| error.clone())
            }
        } else {
            channel_check = None;
            None
        };
        let change = {
            let Ok(mut state) = service.state.lock() else {
                return;
            };
            if state.generation != generation || !state.snapshot.enabled {
                continue;
            }
            let before = state.snapshot.clone();
            let newest = match poll {
                Ok((NotificationStatus::Listening, current)) => {
                    state.snapshot.status = NotificationStatus::Listening;
                    state.snapshot.error = channel_error;
                    let newest = state.buffer.sync(current);
                    state.snapshot.items = state.buffer.items.clone();
                    newest
                }
                Ok((status, _)) => {
                    state.buffer = NotificationBuffer::default();
                    state.snapshot.items.clear();
                    state.snapshot.status = status;
                    state.snapshot.error = None;
                    None
                }
                Err(error) => {
                    state.snapshot.status = NotificationStatus::Error;
                    state.snapshot.error = Some(error);
                    None
                }
            };
            if state.snapshot != before {
                state.snapshot.revision += 1;
                Some((state.snapshot.clone(), newest))
            } else {
                None
            }
        };
        if let Some((snapshot, newest)) = change {
            publish(&service, snapshot, newest);
        }
        // Windows' own change event (or a settings change from the window) wakes this at once;
        // the timeout is only the fallback for a listener that never reports anything.
        let wait = wait_between_reads(channel.as_ref().is_some_and(Channel::wakes_on_events));
        if receiver.recv_timeout(wait) == Err(mpsc::RecvTimeoutError::Disconnected) {
            return;
        }
    }
}

/// How long the monitor may sit still before it reads the store on its own.
///
/// With a change event it is only the safety net; without one — an unpackaged app is refused the
/// subscription on this Windows — it *is* the read interval, one second, as it was before.
pub(super) fn wait_between_reads(wakes_on_events: bool) -> Duration {
    if wakes_on_events { SAFETY_POLL } else { POLL_FALLBACK }
}

/// What is wrong with QQ's notification channel, repairing its registration once per generation.
fn check_channel(
    service: &NotificationService,
    generation: u64,
    repair_generation: &mut Option<u64>,
    repair_error: &mut Option<String>,
) -> Option<String> {
    let setting = qq_notification_setting();
    if *repair_generation != Some(generation) {
        *repair_error = None;
        if should_repair_registration(&setting) {
            *repair_generation = Some(generation);
            *repair_error = repair_registration(service, generation).err();
        }
    }
    let setting = qq_notification_setting();
    if matches!(setting, Ok(NotificationSetting::Enabled)) {
        *repair_error = None;
        None
    } else {
        repair_error.clone().or_else(|| registration_error(setting))
    }
}

fn qq_notification_setting() -> windows::core::Result<NotificationSetting> {
    let _apartment = win32::com::WinRtApartment::multithreaded()?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(QQ_APP_ID))?.Setting()
}

pub(super) fn should_repair_registration(
    setting: &windows::core::Result<NotificationSetting>,
) -> bool {
    setting
        .as_ref()
        .is_err_and(|error| error.code() == HRESULT(0x80070490u32 as i32))
}

pub(super) fn registration_error(
    setting: windows::core::Result<NotificationSetting>,
) -> Option<String> {
    match setting {
        Ok(NotificationSetting::Enabled) => None,
        Ok(NotificationSetting::DisabledForApplication) => {
            Some("QQ 的 Windows 通知已关闭，请在系统通知设置中开启。".into())
        }
        Ok(NotificationSetting::DisabledForUser) => {
            Some("Windows 系统通知已关闭，请在系统通知设置中开启。".into())
        }
        Ok(NotificationSetting::DisabledByGroupPolicy) => {
            Some("管理员策略禁止了 Windows 通知，请联系管理员。".into())
        }
        Ok(_) => Some("QQ 当前安装方式不支持系统通知，请检查或更新 QQ。".into()),
        Err(error) => Some(format!(
            "QQ 通知通道尚未就绪，请确认已安装 QQ；可关闭后重新开启接收以重试：{error}"
        )),
    }
}

fn repair_registration(service: &NotificationService, generation: u64) -> Result<(), String> {
    // Embed the existing guarded repair so installed builds do not depend on the repository.
    let script = format!("& {{\n{REGISTRATION_REPAIR}\n}} -Repair");
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let system_root = std::env::var_os("SystemRoot").ok_or("无法定位 Windows 通知修复工具。")?;
    let executable =
        PathBuf::from(system_root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut child = Command::new(executable)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &STANDARD.encode(bytes),
        ])
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("无法启动 QQ 通知自动修复：{error}"))?;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err(
                        "QQ 通知自动修复未完成，请确认 QQ 安装正常；可关闭后重新开启接收以重试。"
                            .into(),
                    )
                }
            }
            Ok(None)
                if Instant::now() < deadline
                    && service.state.lock().is_ok_and(|state| {
                        state.snapshot.enabled && state.generation == generation
                    }) =>
            {
                thread::sleep(Duration::from_millis(50))
            }
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(match result {
                    Err(error) => format!("QQ 通知自动修复失败：{error}"),
                    _ => "QQ 通知自动修复已停止或超时，可关闭后重新开启接收以重试。".into(),
                });
            }
        }
    }
}

/// Reads the notification store once. Enters the Windows Runtime on its own: the monitor thread
/// passes its live listener to [`read_from`] instead. Kept for the live check in the tests.
#[cfg(test)]
pub(super) fn read_notifications() -> Result<(NotificationStatus, Vec<QQNotification>), String> {
    let _apartment = win32::com::WinRtApartment::multithreaded()
        .map_err(|error| format!("无法初始化 Windows 通知：{error}"))?;
    let listener = UserNotificationListener::Current()
        .map_err(|error| format!("Windows 通知监听不可用：{error}"))?;
    read_from(&listener)
}

/// Reads the notification store through a listener that is already open.
fn read_from(
    listener: &UserNotificationListener,
) -> Result<(NotificationStatus, Vec<QQNotification>), String> {
    let access = listener
        .GetAccessStatus()
        .map_err(|error| format!("无法检查通知权限：{error}"))?;
    if access != UserNotificationListenerAccessStatus::Allowed {
        return Ok((
            if access == UserNotificationListenerAccessStatus::Denied {
                NotificationStatus::PermissionDenied
            } else {
                NotificationStatus::PermissionRequired
            },
            Vec::new(),
        ));
    }
    let operation = listener
        .GetNotificationsAsync(NotificationKinds::Toast)
        .map_err(|error| format!("无法读取 Windows 通知：{error}"))?;
    let notifications =
        pollster::block_on(operation).map_err(|error| format!("无法读取 Windows 通知：{error}"))?;
    let mut items = Vec::new();
    for notification in notifications {
        if let Ok(Some(item)) = read_qq_notification(&notification) {
            items.push(item);
        }
    }
    Ok((NotificationStatus::Listening, items))
}

/// The record for a notification whose text was not read: it says only that one arrived,
/// with the same id and time the full record would have carried.
pub(super) fn arrival_only(id: u32, ticks: i64) -> QQNotification {
    QQNotification {
        id: format!("QQ:{id}:{ticks}"),
        title: String::new(),
        body: String::new(),
        created_at: ticks / 10_000 - 11_644_473_600_000,
    }
}

fn read_qq_notification(
    notification: &UserNotification,
) -> windows::core::Result<Option<QQNotification>> {
    if notification.AppInfo()?.AppUserModelId()? != QQ_APP_ID {
        return Ok(None);
    }
    let id = notification.Id()?;
    let ticks = notification.CreationTime()?.UniversalTime;
    // Settings -> Messages -> "only unread": keep that a message arrived and nothing
    // else. The text elements are never fetched, so the words stay in Windows'
    // notification store instead of passing through this process at all.
    if !crate::message_reading::reads_content() {
        return Ok(Some(arrival_only(id, ticks)));
    }
    let mut parts = Vec::new();
    if let Ok(elements) = notification
        .Notification()?
        .Visual()?
        .GetBinding(&KnownNotificationBindings::ToastGeneric()?)
        .and_then(|binding| binding.GetTextElements())
    {
        for element in elements {
            if let Ok(text) = element.Text() {
                parts.push(text.to_string_lossy());
            }
        }
    }
    if parts.first().is_some_and(|title| title == DIAGNOSTIC_TITLE)
        && parts.iter().any(|body| body == DIAGNOSTIC_BODY)
    {
        return Ok(None);
    }
    // The island's own channel test.
    if parts
        .first()
        .is_some_and(|title| title == "Fly Island QQ notification diagnostic")
        && parts
            .iter()
            .any(|body| body == "Local channel test; this is not a chat message.")
    {
        return Ok(None);
    }
    let title = parts
        .first()
        .filter(|text| !text.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| "QQ".into());
    let body = parts.iter().skip(1).cloned().collect::<Vec<_>>().join("\n");
    Ok(Some(QQNotification {
        id: format!("QQ:{id}:{ticks}"),
        title: title.chars().take(120).collect(),
        body: body.chars().take(1000).collect(),
        created_at: ticks / 10_000 - 11_644_473_600_000,
    }))
}
