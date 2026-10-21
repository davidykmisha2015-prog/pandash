use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub prompt: PromptConfig,
    #[serde(default)]
    pub error: ErrorConfig,
    #[serde(default)]
    pub commands: HashMap<String, CommandAction>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromptConfig {
    #[serde(default = "default_prompt_format")]
    pub format: String,
}

fn default_prompt_format() -> String {
    "{cyan} {green}[{user}:{dir}]{reset}{yellow}{git_branch}{reset} {blue}➜{reset} ".to_string()
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            format: default_prompt_format(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorConfig {
    #[serde(default = "default_not_found_format")]
    pub not_found_format: String,

    #[serde(default = "default_cd_format")]
    pub cd_format: String,
}

fn default_not_found_format() -> String {
    "{red}󰅚 panda: команда '{command}' не знайдена ({error}){reset}".to_string()
}

fn default_cd_format() -> String {
    "{red}󰅚 cd: каталог '{dir}' недоступний ({error}){reset}".to_string()
}

impl Default for ErrorConfig {
    fn default() -> Self {
        Self {
            not_found_format: default_not_found_format(),
            cd_format: default_cd_format(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum CommandAction {
    Simple(String),
    Detailed {
        action: String,
        #[serde(default)]
        r#type: Option<String>,
        #[serde(default)]
        description: Option<String>,
    },
}

pub const DEFAULT_CONFIG_CONTENT: &str = r#"# ==============================================================================
#                 Конфігурація Panda Shell (~/.config/panda/config.toml)
# ==============================================================================

[prompt]
# Формат рядка запрошення.
# Підтримуються змінні:
#   {user}        - ім'я користувача
#   {dir}         - поточний каталог (~ для домашнього)
#   {dir_name}    - назва поточної папки
#   {git_branch}  - поточна гілка Git (з іконкою  ), якщо папка в репозиторії
#   {hostname}    - ім'я комп'ютера
#
# Підтримуються кольори ANSI:
#   {reset}, {bold}, {dim}, {red}, {green}, {yellow}, {blue}, {magenta}, {cyan}, {white}, {gray}
#
# Підтримка Nerd Fonts (можна вставляти будь-які іконки):
#    ,  , ➜ ,  , 󰅚 , 🐼 , ⚡ ,  тощо.
format = "{cyan} {green}[{user}:{dir}]{reset}{yellow}{git_branch}{reset} {blue}➜{reset} "

[error]
# Вигляд повідомлення, коли введена команда не знайдена
# Змінні: {command}, {error}, кольори ANSI та Nerd Fonts символи
not_found_format = "{red}󰅚 panda: команда '{command}' не знайдена ({error}){reset}"

# Вигляд повідомлення при помилці переходу в папку (cd)
# Змінні: {dir}, {error}, кольори ANSI та Nerd Fonts символи
cd_format = "{red}󰅚 cd: каталог '{dir}' недоступний ({error}){reset}"

# ==============================================================================
#                      Конкретні дії для конкретних команд
# ==============================================================================
# Тут можна налаштувати власні аліаси або кастомні дії.
[commands]
# Прості аліаси:
ll = "ls -la --color=auto"
la = "ls -A --color=auto"
l = "ls -CF --color=auto"
c = "clear"
gs = "git status"
gp = "git push"

# Команда з виведенням власного тексту (type = "text"):
[commands.about]
type = "text"
action = "{cyan}🐼 Panda Shell v0.1.0{reset}\nШвидкий кастомний шелл на Rust з підтримкою Nerd Fonts та TOML-конфігурації!"
description = "Інформація про Panda Shell"

# Кастомна команда для виклику системної утиліти:
[commands.sysinfo]
action = "fastfetch || nitch"
description = "Показати інформацію про систему"
"#;

impl Config {
    /// Повертає шлях до файлу конфігурації.
    /// Перевіряє ./config.toml, потім $XDG_CONFIG_HOME/panda/config.toml або ~/.config/panda/config.toml.
    pub fn get_config_path() -> PathBuf {
        let local_config = Path::new("config.toml");
        if local_config.exists() {
            return local_config.to_path_buf();
        }

        if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
            let path = Path::new(&xdg_config).join("panda").join("config.toml");
            if path.exists() {
                return path;
            }
        }

        if let Ok(home) = env::var("HOME") {
            let path = Path::new(&home).join(".config").join("panda").join("config.toml");
            return path;
        }

        PathBuf::from("config.toml")
    }

    /// Завантажує конфігурацію або створює файл за замовчуванням, якщо його немає.
    pub fn load() -> Self {
        let path = Self::get_config_path();

        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match toml::from_str::<Config>(&content) {
                    Ok(cfg) => return cfg,
                    Err(e) => {
                        eprintln!("\x1b[33m[Попередження] Помилка синтаксису в {}: {}\x1b[0m", path.display(), e);
                        eprintln!("\x1b[33mВикористовуються налаштування за замовчуванням.\x1b[0m");
                    }
                },
                Err(e) => {
                    eprintln!("\x1b[33m[Попередження] Не вдалося прочитати {}: {}\x1b[0m", path.display(), e);
                }
            }
        } else {
            // Створюємо директорію та дефолтний конфіг
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(e) = fs::write(&path, DEFAULT_CONFIG_CONTENT) {
                eprintln!("\x1b[33m[Попередження] Не вдалося створити {}: {}\x1b[0m", path.display(), e);
            }
        }

        match toml::from_str::<Config>(DEFAULT_CONFIG_CONTENT) {
            Ok(cfg) => cfg,
            Err(_) => Config::default(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut commands = HashMap::new();
        commands.insert("ll".to_string(), CommandAction::Simple("ls -la --color=auto".to_string()));
        commands.insert("c".to_string(), CommandAction::Simple("clear".to_string()));
        commands.insert(
            "about".to_string(),
            CommandAction::Detailed {
                action: "🐼 Panda Shell — кастомний шелл на Rust!".to_string(),
                r#type: Some("text".to_string()),
                description: Some("Інформація про шелл".to_string()),
            },
        );

        Self {
            prompt: PromptConfig::default(),
            error: ErrorConfig::default(),
            commands,
        }
    }
}

pub fn replace_colors(input: &str) -> String {
    input
        .replace("{reset}", "\x1b[0m")
        .replace("{bold}", "\x1b[1m")
        .replace("{dim}", "\x1b[2m")
        .replace("{black}", "\x1b[30m")
        .replace("{red}", "\x1b[31m")
        .replace("{green}", "\x1b[32m")
        .replace("{yellow}", "\x1b[33m")
        .replace("{blue}", "\x1b[34m")
        .replace("{magenta}", "\x1b[35m")
        .replace("{cyan}", "\x1b[36m")
        .replace("{white}", "\x1b[37m")
        .replace("{gray}", "\x1b[90m")
}

pub fn unescape_str(input: &str) -> String {
    input
        .replace("\\x1b", "\x1b")
        .replace("\\e", "\x1b")
        .replace("\\033", "\x1b")
        .replace("\\n", "\n")
        .replace("\\t", "\t")
}

pub fn get_git_branch(start_dir: &Path) -> Option<String> {
    let mut current = start_dir.to_path_buf();
    for _ in 0..6 {
        let git_dir = current.join(".git");
        if git_dir.is_dir() {
            let head_file = git_dir.join("HEAD");
            if let Ok(content) = fs::read_to_string(head_file) {
                let trimmed = content.trim();
                if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
                    return Some(branch.to_string());
                } else if !trimmed.is_empty() {
                    return Some(trimmed.chars().take(7).collect());
                }
            }
            return None;
        } else if git_dir.is_file() {
            if let Ok(content) = fs::read_to_string(git_dir) {
                if let Some(git_path) = content.trim().strip_prefix("gitdir:") {
                    let full_git_dir = current.join(git_path.trim());
                    let head_file = full_git_dir.join("HEAD");
                    if let Ok(head_content) = fs::read_to_string(head_file) {
                        let trimmed = head_content.trim();
                        if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
                            return Some(branch.to_string());
                        }
                    }
                }
            }
            return None;
        }

        if !current.pop() {
            break;
        }
    }
    None
}

pub fn render_prompt(prompt_cfg: &PromptConfig, home: &str) -> String {
    let user = env::var("USER").unwrap_or_else(|_| "user".to_string());
    let current_dir = env::current_dir().unwrap_or_default();
    let current_dir_str = current_dir.to_string_lossy();

    let display_dir = if !home.is_empty() && current_dir_str.starts_with(home) {
        current_dir_str.replacen(home, "~", 1)
    } else {
        current_dir_str.to_string()
    };

    let dir_name = current_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "/".to_string());

    let hostname = env::var("HOSTNAME")
        .or_else(|_| env::var("HOST"))
        .unwrap_or_else(|_| "nixos".to_string());

    let git_branch = get_git_branch(&current_dir)
        .map(|b| format!("  {}", b))
        .unwrap_or_default();

    let mut rendered = prompt_cfg.format.clone();
    rendered = rendered.replace("{user}", &user);
    rendered = rendered.replace("{dir}", &display_dir);
    rendered = rendered.replace("{dir_name}", &dir_name);
    rendered = rendered.replace("{home}", home);
    rendered = rendered.replace("{hostname}", &hostname);
    rendered = rendered.replace("{git_branch}", &git_branch);

    rendered = replace_colors(&rendered);
    unescape_str(&rendered)
}

pub fn render_not_found(error_cfg: &ErrorConfig, command: &str, error: &std::io::Error) -> String {
    let mut rendered = error_cfg.not_found_format.clone();
    rendered = rendered.replace("{command}", command);
    rendered = rendered.replace("{error}", &error.to_string());
    rendered = replace_colors(&rendered);
    unescape_str(&rendered)
}

pub fn render_cd_error(error_cfg: &ErrorConfig, dir: &str, error: &std::io::Error) -> String {
    let mut rendered = error_cfg.cd_format.clone();
    rendered = rendered.replace("{dir}", dir);
    rendered = rendered.replace("{error}", &error.to_string());
    rendered = replace_colors(&rendered);
    unescape_str(&rendered)
}
