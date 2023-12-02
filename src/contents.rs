use std::io::stdout;

use crossterm::{
    cursor::{MoveTo, MoveToColumn},
    style::{self, Color, Print},
    terminal::{Clear, ClearType},
    QueueableCommand,
};

extern crate unicode_width;
use unicode_width::UnicodeWidthChar;

/// エディタ領域に表示する文字列を出力する
pub fn print_screen(contents: &str) -> std::io::Result<()> {
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
pub fn get_editor_contents(
    contents: &str,
    editor_area_width: u16,
    editor_area_height: u16,
    cursor_x: u16,
    cursor_y: u16,
) -> String {
    // 行番号の表示に必要な桁数を計算する
    let line_number_digits = contents.lines().count().to_string().len();

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
        // 行番号が前後のエスケープシーケンスによって色が変わるので、
        // 行番号自体の色を
        editor_contents.push_str(&format!(
            "{}{:width$} {}",
            style::SetForegroundColor(Color::DarkGrey),
            line_number,
            style::ResetColor,
            width = line_number_digits,
        ));

        // 行を表示幅に分割したベクタを結合する
        if split_line.is_empty() {
            editor_contents.push('\n');
        } else {
            editor_contents.push_str(split_line[0].as_str());
            editor_contents.push('\n');
        }

        // 分割して2行以上になった場合は、行番号を表示しない
        if 1 < split_line.len() {
            split_line[1..].iter().for_each(|line| {
                editor_contents.push_str(&" ".repeat(line_number_digits + line_number_space));
                editor_contents.push_str(line);
                editor_contents.push('\n');
            });
        }

        line_number += 1;
    }

    // カーソルの位置から表示する領域を計算する
    let (_start_x, start_y, _end_x, end_y) =
        get_display_area(editor_area_width, editor_area_height, cursor_x, cursor_y);

    // エスケープシーケンスを考慮して表示する領域を切り出す
    // x軸方向の切り出しは、行わない(エスケープシーケンスの考慮が面倒なのと、今は必ずおりたたみ表示になるので、y軸方向の切り出しは不要)
    let editor_contents = editor_contents
        .lines()
        .skip(start_y as usize)
        .take((end_y - start_y) as usize)
        .collect::<Vec<&str>>()
        .join("\n");

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
pub fn split_string_by_width(s: &str, width: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut current_width = 0;
    let mut current_line = String::new();

    // エスケープシーケンス中は、文字列の長さを計算しない
    let mut is_ansi_escape_sequence = false;

    for (_i, c) in s.char_indices() {
        if is_escape(c) {
            is_ansi_escape_sequence = true;
            current_line.push(c);
            continue;
        }

        if is_ansi_escape_sequence {
            // エスケープシーケンスの終了を判定する
            if c == 'm' {
                is_ansi_escape_sequence = false;
            }
            current_line.push(c);
            continue;
        }

        if current_width + c.width().unwrap() > width {
            current_width = 0;

            result.push(current_line.clone());
            current_line.clear();
        }

        current_line.push(c);

        current_width += c.width().unwrap();
    }

    result.push(current_line);

    result
}

/// エスケープシーケンスかどうかを判定する
/// # Arguments
/// * `c` - 判定する文字
/// # Returns
/// * `bool` - エスケープシーケンスの場合はtrue、それ以外はfalse
/// # Examples
/// ```
/// let c = '\x1b';
/// let result = is_escape(c);
/// assert_eq!(result, true);
/// ```
/// # Panics
fn is_escape(c: char) -> bool {
    c == '\x1b'
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
    /// エスケープシーケンスが含まれる場合
    /// エスケープシーケンスは、表示幅に含めない
    /// そのため、表示幅を考慮した行の切り出しをする
    /// エスケープシーケンスは、\x1bで始まる
    /// エスケープシーケンスは、mで終わる
    fn test_split_string_by_width_with_escape_sequence() {
        // エスケープシーケンスが含まれる場合
        let contents = "\x1b[31mHello, world!\x1b[0m";
        let width = 5;
        let result = split_string_by_width(contents, width);
        assert_eq!(result, vec!["\x1b[31mHello", ", wor", "ld!\x1b[0m"]);

        // マルチバイト文字列の場合
        let contents = "\x1b[31mあい\x1b[0mう";
        let width = 5;
        let result = split_string_by_width(contents, width);
        // "\x1b[31m" は、表示幅が0だけど、次の"Hello"が表示幅を超えるので、"Hello"は次の行に移動する
        assert_eq!(result, vec!["\x1b[31mあい\x1b[0m", "う"]);
    }

    #[test]
    fn test_get_display_area() {
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
        // TODO 行番号の色を変数にして、テストを書きやすくする
        assert_eq!(
            result,
            "\x1b[38;5;8m1 \x1b[0mAlice\n\x1b[38;5;8m2 \x1b[0mBob\n\x1b[38;5;8m3 \x1b[0mCarol\n\x1b[38;5;8m4 \x1b[0mDave\n\x1b[38;5;8m5 \x1b[0mEve"
        );
    }
}
