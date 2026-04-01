use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub enum Timeout {
    Milliseconds(u32),
}

#[derive(Debug, Clone, Default)]
pub struct Notification {
    appname: String,
    summary: String,
    body: String,
    timeout: Option<Timeout>,
}

impl Notification {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn appname(mut self, value: impl Into<String>) -> Self {
        self.appname = value.into();
        self
    }

    pub fn summary(mut self, value: impl Into<String>) -> Self {
        self.summary = value.into();
        self
    }

    pub fn body(mut self, value: impl Into<String>) -> Self {
        self.body = value.into();
        self
    }

    pub fn timeout(mut self, value: Timeout) -> Self {
        self.timeout = Some(value);
        self
    }

    pub fn show(&self) -> Result<(), String> {
        if !cfg!(target_os = "windows") {
            return Ok(());
        }

        let appname = escape_ps_single_quotes(if self.appname.is_empty() {
            "Acta"
        } else {
            &self.appname
        });
        let summary = escape_ps_single_quotes(if self.summary.is_empty() {
            "Acta"
        } else {
            &self.summary
        });
        let body = escape_ps_single_quotes(&self.body);

        let seconds = match self.timeout {
            Some(Timeout::Milliseconds(ms)) => (ms / 1000).max(1),
            None => 8,
        };

        let script = format!(
            r#"
Add-Type -AssemblyName System.Runtime.WindowsRuntime | Out-Null
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
$template = [Windows.UI.Notifications.ToastTemplateType]::ToastText02
$xml = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent($template)
$textNodes = $xml.GetElementsByTagName('text')
$null = $textNodes.Item(0).AppendChild($xml.CreateTextNode('{summary}'))
$null = $textNodes.Item(1).AppendChild($xml.CreateTextNode('{body}'))
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
$toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds({seconds})
$notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('{appname}')
$notifier.Show($toast)
"#
        );

        let status = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .status()
            .map_err(|error| error.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("PowerShell завершився з кодом {status}"))
        }
    }
}

fn escape_ps_single_quotes(value: &str) -> String {
    value.replace('\'', "''")
}
