use std::{
    io::{stdout, Read, Write},
    time::Duration,
};

use clap::Parser;

use crossterm::{
    cursor::{Hide, Show},
    event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};

extern crate unicode_width;

use clap::CommandFactory;

mod contents;
mod status_bar;

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // 端末のサイズを取得する
    let (mut term_width, mut term_height) = terminal::size()?;

    let original_contents = match get_contents(args.file.clone()) {
        Ok(contents) => contents,
        Err(e) => {
            // 標準入力がなく、ファイルを指定していない場合はヘルプを表示するため、標準エラー出力には何も出力しない
            if (e.kind() == std::io::ErrorKind::Other) && (e.to_string() == "No input file") {
            } else {
                eprintln!("{}", e);
            }

            std::process::exit(1);
        }
    };

    queue!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    queue!(stdout(), Hide)?;

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        queue!(stdout(), Show).unwrap();
        disable_raw_mode().unwrap();
        queue!(stdout(), LeaveAlternateScreen).unwrap();
        stdout().flush().unwrap();
        default_hook(panic_info);
    }));

    execute!(stdout(), terminal::Clear(terminal::ClearType::All))?;
    let status_bar_height = 1;
    let status_bar_width = term_width;

    let mut status_bar = status_bar::StatusBar::new(
        status_bar_width,
        status_bar_height,
        0,
        term_height - status_bar_height,
    );

    let status_bar_encoding =
        status_bar::StatusBarItem::new("encoding".to_string(), "UTF-8".to_string());

    status_bar.add_item(status_bar_encoding);

    // エディタ領域に表示する文字列を取得する
    let cursor_x = 0;
    let mut cursor_y = 0;
    let mut editor_height = term_height - status_bar_height;
    let mut contents = contents::Contents::new(
        original_contents.clone(),
        term_width,
        editor_height,
        0,
        0,
        cursor_x,
        cursor_y,
    );

    let status_bar_line = status_bar::StatusBarItem::new(
        "line".to_string(),
        "ln ".to_string() + (cursor_y + 1).to_string().as_str(),
    );
    status_bar.add_item(status_bar_line);

    contents.print()?;
    status_bar.print();
    stdout().flush()?;

    loop {
        let event = read()?;

        // イベントを読み捨てるため、pollを呼び出す
        while poll(Duration::from_secs(0))? {
            let _ = read()?;
        }

        // Ctrl + W で抜ける
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('w'),
                modifiers: KeyModifiers::CONTROL,
                kind: _,
                state: _,
            }) => {
                break;
            }
            // Upキーでカーソルを上に移動する
            Event::Key(KeyEvent {
                code: KeyCode::Up,
                modifiers: _,
                kind: _,
                state: _,
            }) => {
                cursor_y = if cursor_y == 0 { 0 } else { cursor_y - 1 };

                let mut contents = contents::Contents::new(
                    original_contents.clone(),
                    term_width,
                    editor_height,
                    0,
                    0,
                    cursor_x,
                    cursor_y,
                );
                contents.print()?;

                // 表示するときに再計算されるので、cursor_yを更新する
                cursor_y = contents.cursor_y;

                let status_bar_line = status_bar::StatusBarItem::new(
                    "line".to_string(),
                    "ln ".to_string() + (cursor_y + 1).to_string().as_str(),
                );
                status_bar.add_item(status_bar_line);

                status_bar.print();
                stdout().flush()?;
            }

            // Downキーでカーソルを下に移動する
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                modifiers: _,
                kind: _,
                state: _,
            }) => {
                cursor_y += 1;

                let mut contents = contents::Contents::new(
                    original_contents.clone(),
                    term_width,
                    editor_height,
                    0,
                    0,
                    cursor_x,
                    cursor_y,
                );
                contents.print()?;

                // 表示するときに再計算されるので、cursor_yを更新する
                cursor_y = contents.cursor_y;

                let status_bar_line = status_bar::StatusBarItem::new(
                    "line".to_string(),
                    "ln ".to_string() + (cursor_y + 1).to_string().as_str(),
                );
                status_bar.add_item(status_bar_line);

                status_bar.print();
                stdout().flush()?;
            }
            // RightキーとLeftキーでX軸方向でカーソルを移動する機能は未実装
            // 理由: 今は必ずおりたたみ表示になるので、X軸方向でカーソルを移動する機能は不要
            Event::FocusGained => todo!(),
            Event::FocusLost => todo!(),
            Event::Mouse(_) => todo!(),
            Event::Paste(_) => todo!(),
            Event::Resize(columns, rows) => {
                term_width = columns;
                term_height = rows;
                editor_height = term_height - status_bar_height;

                let mut contents = contents::Contents::new(
                    original_contents.clone(),
                    term_width,
                    editor_height,
                    0,
                    0,
                    cursor_x,
                    cursor_y,
                );

                status_bar.width = term_width;
                status_bar.y_start = term_height - status_bar_height;

                contents.print()?;

                // 表示するときに再計算されるので、cursor_yを更新する
                cursor_y = contents.cursor_y;

                let status_bar_line = status_bar::StatusBarItem::new(
                    "line".to_string(),
                    "ln ".to_string() + (cursor_y + 1).to_string().as_str(),
                );
                status_bar.add_item(status_bar_line);

                status_bar.print();
                stdout().flush()?;
            }
            _ => {}
        }
    }

    queue!(stdout(), Show)?;

    disable_raw_mode()?;

    queue!(stdout(), LeaveAlternateScreen)?;

    stdout().flush()?;
    Ok(())
}

/// ファイルの内容を取得する
/// # Arguments
/// * `file` - ファイル名
/// # Returns
/// * `Result<String, std::io::Error>` - ファイルの内容を取得できた場合は、`Ok(String)`を返す
/// # Examples
/// ```
/// let mut contents = String::new();
/// let args = Args::parse();
/// let result = get_contents(args, &mut contents);
/// assert_eq!(result, Ok(()));
/// ```
/// # Panics
/// * `args.file`が存在しない場合は、エラーを表示して終了する
/// # Notes
/// | `file`       | `file`の存在       | 標準入力  | 返り値                   |
/// | :----------- | :----------------- | :-------- | :----------------------- |
/// | `Some(file)` | 存在する           | あり/なし | `file`の内容             |
/// | `Some(file)` | 存在しない         | あり/なし | エラーを表示して終了する |
/// | `None`       |                    | あり      | 標準入力の内容           |
/// | `None`       |                    | なし      | エラーを表示して終了する |
fn get_contents(file: Option<String>) -> Result<String, std::io::Error> {
    let mut contents = String::new();
    match file {
        Some(file) => {
            // ファイルが存在しない場合は、エラーを表示して終了する
            match std::fs::read_to_string(&file) {
                Ok(file_contents) => contents = file_contents,
                Err(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("{}: No such file or directory", file),
                    ));
                }
            }
        }
        None => {
            if atty::is(atty::Stream::Stdin) {
                let mut args = Args::command();
                // 装飾付きの文字でヘルプを表示したいので、ここで`print_help`を呼び出す
                args.print_help().unwrap();
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No input file",
                ));
            } else {
                std::io::stdin().read_to_string(&mut contents)?;
            }
        }
    };
    Ok(contents)
}

#[derive(Debug, Parser)]
#[clap(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    arg_required_else_help = false,
)]
struct Args {
    /// File to print. If no FILE is specified, read standard input.
    #[clap()]
    file: Option<String>,
}
