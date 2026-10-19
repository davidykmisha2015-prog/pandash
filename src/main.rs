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
            // Вимикаємо канонічний режим (ICANON) та відлуння за замовчуванням (ECHO),
            // щоб читати байти самостійно і контролювати вивід на екран
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
        // Коли виходимо з області видимості, повертаємо старі налаштування термінала
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
        }
    }
}

fn main() {
    // Безкінечний цикл нашого шелла
    ctrlc::set_handler(|| println!()).unwrap();
    
    let mut history: Vec<String> = Vec::new();

    loop {
        // Дізнаємося нік користувача
        let prompt = env::var("USER").unwrap_or_else(|_| "user".to_string());
        let current_dir = env::current_dir().unwrap_or_default();
        let r#home = env::var("HOME").unwrap_or_else(|_| "home".to_string());

        // 1. Виводимо красиве запрошення (у звичайному режимі виводу)
        print!("\x1b[32m[{}:{}]\x1b[0m > ", prompt, current_dir.display());
        io::stdout().flush().unwrap();

        let mut history_index = 0;
        history_index = history.len();
        let _ = history_index; // щоб компілятор не сварився

        // 2. Читання вводу в Raw Mode (по байтах)
        let mut input_bytes = Vec::new();
        
        {
            // Увімкнули Raw Mode на час введення рядка
            let _raw_guard = RawModeGuard::enable().unwrap();
            let mut stdin = io::stdin().bytes();

            loop {
                if let Some(Ok(byte)) = stdin.next() {
                    match byte {
                        // Enter (перенос рядка)
                        b'\n' | b'\r' => {
                            print!("\r\n");
                            io::stdout().flush().unwrap();
                            break;
                        }
                        27 => {
                            if let Some(Ok(91)) = stdin.next() {
                                if let Some(Ok(direction)) = stdin.next() {
                                    match direction {
                                       65 => {
                                        // Стрілочка вгору (йдемо в минуле історії)
                                        if !history.is_empty() && history_index > 0 {
                                            history_index -= 1;
                                            input_bytes = history[history_index].bytes().collect();
                                            // 1. Формуємо повний промпт у змінну
                                            let prompt = format!(
                                            "\x1b[32m[{}:{}]\x1b[0m > ",
                                            env::var("USER").unwrap_or_else(|_| "user".to_string()),
                                            current_dir.display()
                                        );

                                            // Виводимо його на екран
                                            print!("\r\x1b[K{}{}", prompt, history[history_index]);
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
                                input_bytes.pop();
                                // Стискаємо символ на екрані: переміщення назад, пробіл, назад
                                print!("\x1b[D \x1b[D");
                                io::stdout().flush().unwrap();
                            }
                        }
                        // Звичайні символи (друкуємо і додаємо в буфер)
                        b if b >= 32 && b < 127 => {
                            input_bytes.push(b);
                            print!("{}", b as char);
                            io::stdout().flush().unwrap();
                        }
                        _ => {}
                    }
                }
            }
        } // Тут _raw_guard знищується (Drop), і термінал автоматично повертається у звичний режим на час виконання програм!

        let input = String::from_utf8(input_bytes).unwrap_or_default();
        let trimmed = input.trim().to_string();
        
        if !trimmed.is_empty() {
            history.push(trimmed.clone());
        }

        // Замінюємо всі "~" у введеному тексті на реальний шлях додому
        let input = input.replace("~", &home);

        // Прибираємо зайві пробіли та розбиваємо команду
        let mut parts = input.trim().split_whitespace();
        
        let command = match parts.next() {
            Some(cmd) => cmd,
            None => continue,
        };

        let args = parts;

        // 3. Перевіряємо вбудовані команди та звичайні програми
        match command {
            "exit" => {
                println!("Вихід із шелла...");
                break;
            }
            "cd" => {
                let new_dir = args.into_iter().next().unwrap_or(home.as_str());
                let root = Path::new(new_dir);
                
                if let Err(e) = env::set_current_dir(root) {
                    eprintln!("cd: {}: {}", new_dir, e);
                }
            }
            _ => {
                let child = Command::new(command)
                    .args(args)
                    .spawn();

                match child {
                    Ok(mut child_process) => {
                        let _ = child_process.wait();
                    }
                    Err(e) => {
                        eprintln!("katana-sh: команда '{}' не знайдена ({})", command, e);
                    }
                }
            }
        }
    }
}
