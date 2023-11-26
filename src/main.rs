use std::{
    f32::consts::E,
    io::{stdout, Read, Stdout, Write},
    thread::sleep,
    time::Duration,
};

use clap::{error, Parser};

use crossterm::{
    cursor::{self, DisableBlinking, EnableBlinking, Hide, MoveDown, MoveTo, MoveToColumn, Show},
    event::{
        poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute, queue,
    style::{self, Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor, Stylize},
    terminal::{
        self, disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen, SetTitle,
    },
    ExecutableCommand, QueueableCommand,
};

extern crate unicode_width;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use clap::CommandFactory;

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // 端末のサイズを取得する
    let (term_width, term_height) = terminal::size()?;

    let mut contents = String::new();

    match get_contents(args.file, &mut contents) {
        Ok(_) => {}
        Err(e) => {
            // 標準入力がなく、ファイルを指定していない場合はヘルプを表示するため、標準エラー出力には何も出力しない
            if (e.kind() == std::io::ErrorKind::Other) && (e.to_string() == "No input file") {
            } else {
                eprintln!("{}", e);
            }

            std::process::exit(1);
        }
    }

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

    // エディタ領域に表示する文字列を取得する
    let mut cursor_x = 0;
    let mut cursor_y = 0;
    let editor_contents =
        get_editor_contents(&contents, term_width, term_height, cursor_x, cursor_y);

    print_screen(&editor_contents)?;

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
                // エディタ領域に表示する文字列を取得する
                let mut editor_contents =
                    get_editor_contents(&contents, term_width, term_height, cursor_x, cursor_y);

                if editor_contents.lines().count() < term_height as usize {
                    cursor_y = if cursor_y == 0 { 0 } else { cursor_y + 1 };
                    editor_contents =
                        get_editor_contents(&contents, term_width, term_height, cursor_x, cursor_y);
                }

                print_screen(&editor_contents)?;
            }
            // Downキーでカーソルを下に移動する
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                modifiers: _,
                kind: _,
                state: _,
            }) => {
                cursor_y += 1;
                // エディタ領域に表示する文字列を取得する
                let mut editor_contents =
                    get_editor_contents(&contents, term_width, term_height, cursor_x, cursor_y);

                if editor_contents.lines().count() < term_height as usize {
                    cursor_y -= 1;
                    editor_contents =
                        get_editor_contents(&contents, term_width, term_height, cursor_x, cursor_y);
                }

                print_screen(&editor_contents)?;
            }
            // RightキーとLeftキーでX軸方向でカーソルを移動する機能は未実装
            // 理由: 今は必ずおりたたみ表示になるので、X軸方向でカーソルを移動する機能は不要
            Event::FocusGained => todo!(),
            Event::FocusLost => todo!(),
            Event::Mouse(_) => todo!(),
            Event::Paste(_) => todo!(),
            Event::Resize(_, _) => todo!(),
            _ => {}
        }
    }

    queue!(stdout(), Show)?;

    disable_raw_mode()?;

    queue!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// ファイルの内容を取得する
/// # Arguments
/// * `file` - ファイル名
/// * `contents` - ファイルの内容
/// # Returns
/// * `Result<(), std::io::Error>` - ファイルの内容を取得できた場合は、`Ok(())`を返す
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
fn get_contents(file: Option<String>, contents: &mut String) -> Result<(), std::io::Error> {
    match file {
        Some(file) => {
            // ファイルが存在しない場合は、エラーを表示して終了する
            match std::fs::read_to_string(&file) {
                Ok(file_contents) => *contents = file_contents,
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
                std::io::stdin().read_to_string(contents)?;
            }
        }
    };
    Ok(())
}

fn print_screen(contents: &str) -> std::io::Result<()> {
    // RAWモードで出力するので、一行一行出力する

    stdout().queue(MoveTo(0, 0))?;
    stdout().queue(Clear(ClearType::All))?;

    for line in contents.lines() {
        stdout().queue(Print(line))?;

        // 最終行でない場合は、改行する
        if line != contents.lines().last().unwrap() {
            stdout().queue(Print("\n"))?;
        }

        // カーソルを次の行に移動する
        stdout().queue(MoveToColumn(0))?;
    }

    stdout().flush()?;

    Ok(())
}

/// エディタ領域に表示する文字列を取得する
/// # Arguments
/// * `contents` - ファイルの内容
/// * `editor_area_width` - 端末の横幅
/// * `term_height` - 端末の縦幅
/// * `cursor_x` - カーソルの横位置
/// * `cursor_y` - カーソルの縦位置
/// # Returns
/// * `String` - エディタ領域に表示する文字列
/// # Examples
/// ```
/// let contents = std::fs::read_to_string(args.file).unwrap();
/// let (term_width, term_height) = terminal::size()?;
/// let editor_contents = get_editor_contents(contents, term_width, term_height, cursor_x, cursor_y);
/// ```
/// # Panics
///
/// # Notes
/// * `contents`の文字列の長さが`term_width`よりも長い場合は、`term_width`の長さに切り詰める
/// * `contents`の行数が`term_height`よりも多い場合は、`term_height`の行数に切り詰める
/// * `contents`の行数が`term_height`よりも少ない場合は、`term_height`の行数になるまで改行を追加する
/// * `contents`の文字列の長さが`term_width`よりも短い場合は、空白を追加する
/// * `contents`の行数が`term_height`よりも少ない場合は、空白を追加する
fn get_editor_contents(
    contents: &str,
    editor_area_width: u16,
    editor_area_height: u16,
    cursor_x: u16,
    cursor_y: u16,
) -> String {
    // 行番号の表示に必要な桁数を計算する
    let contents_lines = contents.lines().count();
    let line_number_digits = contents_lines.to_string().len();

    // 行番号とコンテンツの間の空白の数
    let line_number_space = 1;

    // 1行の横幅を計算する
    // 1行の横幅 = エディタ領域の横幅 - 行番号の桁数 - 行番号の後の空白(1文字)
    let line_width = editor_area_width as usize - line_number_digits - line_number_space;

    // contentsの各行の文字列の長さがline_widthよりも長い場合は、長い部分を次の行に移動する
    // 次の行に移動した部分の文字列の長さがline_widthよりも長い場合は、さらに次の行に移動する(これを繰り返す)
    let mut editor_contents = String::new();
    let mut line_number = 1;

    for line in contents.lines() {
        // 行を表示幅に分割したベクタを取得する
        let split_line = split_string_by_width(line, line_width);

        // 行番号と後ろの空白を追加する
        editor_contents.push_str(&format!(
            "{:width$} ",
            line_number,
            width = line_number_digits
        ));

        // 行を表示幅に分割したベクタを結合する
        if split_line.is_empty() {
            editor_contents.push('\n');
        } else {
            editor_contents.push_str(split_line[0].as_str());
            editor_contents.push('\n');
        }

        // 分割して2行以上になった場合は、行番号を表示しない
        if split_line.len() > 1 {
            split_line[1..].iter().for_each(|line| {
                editor_contents.push_str(&" ".repeat(line_number_digits + line_number_space));
                editor_contents.push_str(line);
                editor_contents.push('\n');
            });
        }

        line_number += 1;
    }

    // カーソルの位置から表示する領域を計算する
    let (start_x, start_y, end_x, end_y) =
        get_display_area(editor_area_width, editor_area_height, cursor_x, cursor_y);

    // 表示する領域を切り出す
    editor_contents = editor_contents
        .lines()
        .skip(start_y as usize)
        .take(end_y as usize - start_y as usize)
        .map(|line| {
            let mut line = line
                .chars()
                .skip(start_x as usize)
                .take(end_x as usize - start_x as usize)
                .collect::<String>();
            line.push('\n');
            line
        })
        .collect::<String>();

    // 表示する領域の最後の改行を削除する
    editor_contents.pop();

    editor_contents
}

/// 得た得られた表示幅で文字列を分割する
/// # Arguments
/// * `value` - 分割する文字列
/// * `width` - 端末の横幅
/// # Returns
/// * `Vec<String>` - 表示幅で分割した文字列
/// # Examples
/// ```
/// let contents = "Hello, world!";
/// let width = 5;
/// let result = split_string_by_width(contents, width);
/// assert_eq!(result, vec!["Hello", ", wor", "ld!"]);
/// ```
/// # Panics
///
/// # Notes
/// * `contents`の文字列の長さが`width`よりも長い場合は、`width`の長さに切り詰める(これを繰り返す)
fn split_string_by_width(value: &str, width: usize) -> Vec<String> {
    let mut result = Vec::new();

    let mut current_string = "".to_string();

    // 表示幅を考慮した行の切り出しをする
    for i in 0..value.chars().count() {
        let char = value.chars().nth(i).unwrap();

        if char.width().is_none() {
            continue;
        }

        if current_string.width() + char.width().unwrap() <= width {
            current_string.push(char);
        }

        if (current_string.width() == width) || (i == value.chars().count() - 1) {
            result.push(current_string.clone());
            current_string = "".to_string();
        } else {
            let next_char = value.chars().nth(i + 1).unwrap();
            if next_char.width().is_none() {
                continue;
            }
            if width < current_string.width() + next_char.width().unwrap() {
                result.push(current_string.clone());
                current_string = "".to_string();
            }
        }
    }

    result
}

/// カーソル位置から表示する領域を計算する
/// # Arguments
/// * `editor_area_width` - 端末の横幅
/// * `editor_area_height` - 端末の縦幅
/// * `cursor_x` - カーソルの横位置
/// * `cursor_y` - カーソルの縦位置
/// # Returns
/// * `String` - エディタ領域に表示する文字列
/// # Examples
/// ```
/// let contents = "Alice\nBob\nCarol\nDave\nEve\nFrank\nGrace\nHeidi\nIvan";
/// let width = 5;
/// let result = get_display_area(contents, width, 0, 0);
/// assert_eq!(result, (0, 0, 5, 5));
/// ```
/// # Panics
///
/// # Notes
fn get_display_area(
    editor_area_width: u16,
    editor_area_height: u16,
    cursor_x: u16,
    cursor_y: u16,
) -> (u16, u16, u16, u16) {
    // カーソルの位置から表示する領域を計算する
    let start_x = cursor_x;
    let start_y = cursor_y;
    let end_x = start_x + editor_area_width;
    let end_y = start_y + editor_area_height;

    (start_x, start_y, end_x, end_y)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// ASCII文字列の場合
    fn test_split_string_by_width_all_ascii() {
        let contents = "Hello, world!";
        let width = 5;
        let result = split_string_by_width(contents, width);
        assert_eq!(result, vec!["Hello", ", wor", "ld!"]);
    }

    #[test]
    /// ASCII文字列と日本語文字列が混在する場合
    /// 日本語文字列は、2文字分の幅を占有する
    /// そのため、表示幅を考慮した行の切り出しをする
    fn test_split_string_by_width_ascii_and_japanese() {
        let contents = "Hello, 世界!";
        let width = 5;
        let result = split_string_by_width(contents, width);
        // ", 世" は、表示幅が4だけど、次の"界"が表示幅を超えるので、"世"は次の行に移動する
        assert_eq!(result, vec!["Hello", ", 世", "界!"]);
    }

    #[test]
    fn test_get_display_area() {
        let contents = "Alice\nBob\nCarol\nDave\nEve\nFrank\nGrace\nHeidi\nIvan";
        let width = 100;
        let height = 5;
        let result = get_display_area(width, height, 0, 0);
        assert_eq!(result, (0, 0, 100, 5));

        let result = get_display_area(width, height, 0, 1);
        assert_eq!(result, (0, 1, 100, 6));

        let result = get_display_area(width, height, 0, 2);
        assert_eq!(result, (0, 2, 100, 7));
    }

    #[test]
    fn test_get_editor_contents() {
        let contents = "Alice\nBob\nCarol\nDave\nEve\nFrank\nGrace\nHeidi\nIvan";
        let width = 100;
        let height = 5;
        let result = get_editor_contents(contents, width, height, 0, 0);
        // 最後の行を改行すると、表示領域の最後の行が空白になるので、最後の行を改行しないことが重要
        assert_eq!(result, "1 Alice\n2 Bob\n3 Carol\n4 Dave\n5 Eve");
    }
}
