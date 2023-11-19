use std::{
    io::{stdout, Read, Write},
    thread::sleep,
};

use clap::Parser;

use crossterm::{
    cursor::{self, MoveDown, MoveTo},
    event::{read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{self, Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor, Stylize},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
    ExecutableCommand, QueueableCommand,
};
use std::time::Duration as duration;

extern crate unicode_width;
use unicode_width::UnicodeWidthStr;

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let mut stdout = stdout();
    queue!(stdout, EnterAlternateScreen)?;

    // 端末のサイズを取得する
    let (term_width, term_height) = terminal::size()?;

    let contents = std::fs::read_to_string(args.file).unwrap();
    // execute!(stdout, terminal::Clear(terminal::ClearType::All))?;

    // エディタ領域に表示する文字列を取得する
    let editor_contents = get_editor_contents(&contents, term_width, term_height, 0, 0);

    stdout.queue(Print(editor_contents.clone()))?;

    stdout.flush()?;

    sleep(duration::from_secs(1));

    queue!(stdout, LeaveAlternateScreen)?;
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
        // 行番号と後ろの空白を追加する
        editor_contents.push_str(&format!(
            "{:width$} ",
            line_number,
            width = line_number_digits
        ));

        // editor_area_widthよりも長い場合は、line_widthの長さに切り詰める
        if line.width() > line_width {
            editor_contents.push_str(&line[..line_width]);
        } else {
            editor_contents.push_str(line);
            editor_contents.push('\n');
            line_number += 1;

            continue;
        }

        // editor_area_widthよりも長い場合は、次の行に移動する
        // 行番号は表示せず、空白を追加する
        split_string_by_width(&line[line_width..], line_width)
            .iter()
            .for_each(|line| {
                editor_contents.push_str(&" ".repeat(line_number_digits + line_number_space));
                editor_contents.push_str(line);
                editor_contents.push('\n');
            });

        line_number += 1;
    }

    // // editor_area_heightよりも少ない場合は、空白を追加する
    // if contents_lines < editor_area_height as usize {
    //     editor_contents.push_str(&"\n".repeat(editor_area_height as usize - contents_lines - 1));
    // }

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
    let mut remaining_string = value;

    while remaining_string.width() > width {
        // 表示幅を考慮した行の切り出しをする
        for (i, c) in remaining_string.char_indices() {
            if i == width {
                result.push(remaining_string[..i].to_string());
                remaining_string = &remaining_string[i..];
                break;
            }
        }

        remaining_string = &remaining_string[width..];
    }

    result.push(remaining_string.to_string());

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
/// let contents = a;
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
    let start_x = if cursor_x > editor_area_width / 2 {
        cursor_x - editor_area_width / 2
    } else {
        0
    };
    let start_y = if cursor_y > editor_area_height / 2 {
        cursor_y - editor_area_height / 2
    } else {
        0
    };
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
    arg_required_else_help = true,
)]

struct Args {
    /// File
    #[clap(required = true)]
    file: String,
}
