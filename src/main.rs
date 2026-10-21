mod config;

use config::{render_cd_error, render_not_found, render_prompt, replace_colors, unescape_str, CommandAction, Config};
use std::env;
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::process::Command;

// Структура для автоматичного ввімкнення та вимкнення Raw Mode (через Drop)
struct RawModeGuard {
    fd: i32,
    original: libc::termios,
}

impl RawModeGuard {
    fn enable() -> io::Result<Self> {
        let fd = io::stdin().as_raw_fd();
        unsafe {
            let mut original: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(fd, &mut original) != 0 {
                return Err(io::Error::last_os_error());
            }

            let mut raw = original;
            // Вимикаємо канонічний режим (ICANON) та відлуння (ECHO)
            raw.c_lflag &= !(libc::ICANON | libc::ECHO);

            if libc::tcsetattr(fd, libc::TCSANOW, &raw) != 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(Self { fd, original })
        }
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
        }
    }
}

fn execute_custom_action(action: &str, extra_args: &[String]) {
    let script = if extra_args.is_empty() {
        action.to_string()
    } else {
        format!("{} \"$@\"", action)
    };

    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(script).arg("_");
    cmd.args(extra_args);

    match cmd.spawn() {
        Ok(mut child) => {
            let _ = child.wait();
        }
        Err(e) => {
            eprintln!("\x1b[31mПомилка виконання дії '{}': {}\x1b[0m", action, e);
        }
    }
}

fn main() {
    ctrlc::set_handler(|| println!()).unwrap();

    let mut config = Config::load();
    let home = env::var("HOME").unwrap_or_else(|_| "/home".to_string());
    let mut history: Vec<String> = Vec::new();
    let is_tty = unsafe { libc::isatty(io::stdin().as_raw_fd()) == 1 };

    loop {
        // 1. Формуємо та виводимо динамічний промпт із конфігурації
        let prompt_str = render_prompt(&config.prompt, &home);
        print!("{}", prompt_str);
        io::stdout().flush().unwrap();

        let input: String;

        if is_tty {
            let mut history_index = history.len();
            let mut input_bytes = Vec::new();

            // 2. Читання вводу в Raw Mode (по байтах)
            {
                let _raw_guard = RawModeGuard::enable().ok();
                let mut stdin = io::stdin().bytes();

                loop {
                    if let Some(Ok(byte)) = stdin.next() {
                        match byte {
                            // Enter
                            b'\n' | b'\r' => {
                                print!("\r\n");
                                io::stdout().flush().unwrap();
                                break;
                            }
                            // Ctrl+D на порожньому рядку — вихід
                            4 => {
                                if input_bytes.is_empty() {
                                    print!("\r\n");
                                    io::stdout().flush().unwrap();
                                    println!("Вихід із шелла...");
                                    return;
                                }
                            }
                            // Escape-послідовності (стрілки)
                            27 => {
                                if let Some(Ok(91)) = stdin.next() {
                                    if let Some(Ok(direction)) = stdin.next() {
                                        match direction {
                                            // Стрілка вгору (в минуле історії)
                                            65 => {
                                                if !history.is_empty() && history_index > 0 {
                                                    history_index -= 1;
                                                    input_bytes = history[history_index].as_bytes().to_vec();
                                                    let current_prompt = render_prompt(&config.prompt, &home);
                                                    print!("\r\x1b[K{}{}", current_prompt, history[history_index]);
                                                    io::stdout().flush().unwrap();
                                                }
                                            }
                                            // Стрілка вниз (в майбутнє історії)
                                            66 => {
                                                if !history.is_empty() && history_index < history.len() {
                                                    history_index += 1;
                                                    let current_prompt = render_prompt(&config.prompt, &home);
                                                    if history_index < history.len() {
                                                        input_bytes = history[history_index].as_bytes().to_vec();
                                                        print!("\r\x1b[K{}{}", current_prompt, history[history_index]);
                                                    } else {
                                                        input_bytes.clear();
                                                        print!("\r\x1b[K{}", current_prompt);
                                                    }
                                                    io::stdout().flush().unwrap();
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            // Backspace (127 або 8)
                            127 | 8 => {
                                if !input_bytes.is_empty() {
                                    if let Ok(mut s) = String::from_utf8(input_bytes.clone()) {
                                        if s.pop().is_some() {
                                            input_bytes = s.into_bytes();
                                            print!("\x1b[D \x1b[D");
                                            io::stdout().flush().unwrap();
                                        }
                                    } else {
                                        input_bytes.pop();
                                        print!("\x1b[D \x1b[D");
                                        io::stdout().flush().unwrap();
                                    }
                                }
                            }
                            // Звичайні байти (включаючи повну підтримку UTF-8 та Nerd Fonts)
                            b if b >= 32 => {
                                input_bytes.push(b);
                                let _ = io::stdout().write_all(&[b]);
                                let _ = io::stdout().flush();
                            }
                            _ => {}
                        }
                    } else {
                        break;
                    }
                }
            } // _raw_guard скидається при виході з блоку

            input = String::from_utf8(input_bytes).unwrap_or_default();
        } else {
            // Неінтерактивний режим (пайпи, скрипти тощо)
            let mut line = String::new();
            if io::stdin().read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            input = line;
        }

        let trimmed = input.trim().to_string();

        if !trimmed.is_empty() {
            history.push(trimmed);
        }

        // Замінюємо всі "~" у введеному тексті на шлях до домашньої папки
        let input = input.replace('~', &home);

        let mut parts = input.trim().split_whitespace();
        let command = match parts.next() {
            Some(cmd) => cmd,
            None => continue,
        };

        let args: Vec<String> = parts.map(|s| s.to_string()).collect();

        // 3. Перевірка кастомних дій із конфігурації (аліаси / дії)
        if let Some(custom) = config.commands.get(command).cloned() {
            match custom {
                CommandAction::Simple(action_str) => {
                    let action_trimmed = action_str.trim();
                    if action_trimmed == "exit" {
                        println!("Вихід із шелла...");
                        break;
                    }
                    if let Some(dir) = action_trimmed.strip_prefix("cd ") {
                        let root = Path::new(dir.trim());
                        if let Err(e) = env::set_current_dir(root) {
                            eprintln!("{}", render_cd_error(&config.error, dir.trim(), &e));
                        }
                        continue;
                    }
                    execute_custom_action(&action_str, &args);
                    continue;
                }
                CommandAction::Detailed { action, r#type, .. } => {
                    match r#type.as_deref().unwrap_or("exec") {
                        "text" | "print" => {
                            let msg = unescape_str(&replace_colors(&action));
                            println!("{}", msg);
                            continue;
                        }
                        "reload" => {
                            config = Config::load();
                            let msg = unescape_str(&replace_colors(&action));
                            println!("{}", msg);
                            continue;
                        }
                        _ => {
                            let action_trimmed = action.trim();
                            if action_trimmed == "exit" {
                                println!("Вихід із шелла...");
                                break;
                            }
                            execute_custom_action(&action, &args);
                            continue;
                        }
                    }
                }
            }
        }

        // 4. Вбудовані команди шелла
        match command {
            "exit" => {
                println!("Вихід із шелла...");
                break;
            }
            "reload" => {
                config = Config::load();
                println!("\x1b[32m🐼 Конфігурацію оновлено з файлу {}\x1b[0m", Config::get_config_path().display());
            }
            "cd" => {
                let new_dir = args.first().map(|s| s.as_str()).unwrap_or(&home);
                let root = Path::new(new_dir);

                if let Err(e) = env::set_current_dir(root) {
                    eprintln!("{}", render_cd_error(&config.error, new_dir, &e));
                }
            }
            _ => {
                // 5. Виклик зовнішніх програм
                let child = Command::new(command)
                    .args(&args)
                    .spawn();

                match child {
                    Ok(mut child_process) => {
                        let _ = child_process.wait();
                    }
                    Err(e) => {
                        eprintln!("{}", render_not_found(&config.error, command, &e));
                    }
                }
            }
        }
    }
}
