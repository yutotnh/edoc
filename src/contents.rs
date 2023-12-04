use std::io::stdout;

use crossterm::{
    cursor::{MoveTo, MoveToColumn},
    style::{Attribute, Print},
    terminal::{Clear, ClearType},
    QueueableCommand,
};

extern crate unicode_width;
use unicode_width::UnicodeWidthChar;

/// 分割した文字列
pub struct SplitLine {
    /// 行番号
    pub line_number: u16,
    /// その行の何番目か(0番目から始まる)
    pub line_index: u16,
    /// 行の内容
    pub contents: String,
}

pub struct Contents {
    /// 元の文字列
    pub original_contents: String,
    /// 表示する文字列
    pub contents: Vec<SplitLine>,
    /// 表示する領域の横幅
    pub width: u16,
    /// 表示する領域の縦幅
    pub height: u16,
    /// 表示する領域の開始位置(X座標)
    pub x_start: u16,
    /// 表示する領域の開始位置(Y座標)
    pub y_start: u16,
    /// カーソルの横位置
    pub cursor_x: u16,
    /// カーソルの縦位置
    pub cursor_y: u16,
}

impl Contents {
    /// Contentsを作成する
    pub fn new(
        original_contents: String,
        width: u16,
        height: u16,
        x_start: u16,
        y_start: u16,
        cursor_x: u16,
        cursor_y: u16,
    ) -> Self {
        Self {
            original_contents,
            contents: vec![],
            width,
            height,
            x_start,
            y_start,
            cursor_x,
            cursor_y,
        }
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
    fn split_string_by_width(&self, s: &str, width: u16) -> Vec<String> {
        let mut result = Vec::new();
        let mut current_width = 0;
        let mut current_line = String::new();

        // エスケープシーケンス中は、文字列の長さを計算しない
        let mut is_ansi_escape_sequence = false;

        for (_i, c) in s.char_indices() {
            if self.is_escape(c) {
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

            if current_width + c.width().unwrap() > width as usize {
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
    fn is_escape(&self, c: char) -> bool {
        c == '\x1b'
    }

    /// エディタ領域に表示する文字列を出力する
    pub fn print(&mut self) -> std::io::Result<()> {
        // エディタ領域に表示する文字列を更新する
        self.update_contents();

        // RAWモードで出力するので、一行一行出力する
        stdout().queue(MoveTo(self.x_start, self.y_start))?;
        stdout().queue(Clear(ClearType::All))?;

        // エディタ領域に表示する行数よりも端末の縦幅が小さい場合は、cursor_yを0にして全ての行を表示する
        if self.height > self.contents.len() as u16 {
            self.cursor_y = 0;
        }

        // cursor_yが表示する行数よりも大きい場合は、cursor_yを表示する行数にする
        let max_cursor_y = if self.contents.len() as u16 > self.height {
            self.contents.len() as u16 - self.height
        } else {
            0
        };

        // カーソルの縦位置が表示する行数よりも大きい場合は、カーソルの縦位置を表示する行数にする
        if self.cursor_y > max_cursor_y {
            self.cursor_y = max_cursor_y;
        }

        // 出力する
        let display_area = self.get_display_area();
        let mut current_y = 0;
        let line_number_width = self.contents[self.contents.len() - 1]
            .line_number
            .to_string()
            .len();
        for split_line in &self.contents {
            // 表示する行が表示領域の範囲外の場合は、次の行に移動する
            if current_y < display_area.1 || current_y >= display_area.3 {
                current_y += 1;
                continue;
            }

            stdout().queue(MoveToColumn(self.x_start))?;

            // 1行が分割されている場合があるが、最初だけ行番号を表示する
            if split_line.line_index == 0 {
                // 行番号を表示する
                // 行番号の色は区別しやすいように、薄い色にする
                stdout().queue(Print(Attribute::Dim)).unwrap();
                stdout().queue(Print(format!(
                    "{:>line_number_width$} ",
                    split_line.line_number
                )))?;

                // 行番号の色を薄くするために薄暗い色を設定したので、リセットする
                stdout().queue(Print(Attribute::Reset)).unwrap();
            } else {
                // 行番号の分の空白を表示する
                stdout().queue(Print(" ".repeat(line_number_width + 1)))?;
            }

            // 行の内容を表示する
            stdout().queue(Print(&split_line.contents))?;

            // 次の行を表示することに備えて改行する
            stdout().queue(Print("\n"))?;

            current_y += 1;
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
    fn update_contents(&mut self) {
        // 行番号の表示に必要な桁数を計算する
        let line_number_digits = self.original_contents.lines().count().to_string().len();

        // 行番号とコンテンツの間の空白の数
        let line_number_space = 1;

        // 1行の横幅を計算する
        // 1行の横幅 = エディタ領域の横幅 - 行番号の桁数 - 行番号の後の空白(1文字)
        let line_width = self.width as usize - line_number_digits - line_number_space;

        // contentsの各行の文字列の長さがline_widthよりも長い場合は、長い部分を次の行に移動する
        // 次の行に移動した部分の文字列の長さがline_widthよりも長い場合は、さらに次の行に移動する(これを繰り返す)

        let mut line_number = 1;
        for line in self.original_contents.lines() {
            // 行を表示幅に分割したベクタを取得する
            let split_line = self.split_string_by_width(line, line_width as u16);

            for (i, line) in split_line.iter().enumerate() {
                let split_line = SplitLine {
                    line_number,
                    line_index: i as u16,
                    contents: line.to_string(),
                };
                self.contents.push(split_line);
            }

            line_number += 1;
        }
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
    fn get_display_area(&self) -> (u16, u16, u16, u16) {
        // カーソルの位置から表示する領域を計算する
        let start_x = self.cursor_x;
        let start_y = self.cursor_y;
        let end_x = start_x + self.width;
        let end_y = start_y + self.height;

        (start_x, start_y, end_x, end_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// ASCII文字列の場合
    fn test_split_string_by_width_all_ascii() {
        // インスタンスの値はなんでもいい
        let contents = Contents {
            original_contents: String::new(),
            cursor_x: 0,
            cursor_y: 0,
            width: 0,
            height: 0,
            contents: Vec::new(),
            x_start: 0,
            y_start: 0,
        };

        let string = "Hello, world!";
        let width = 5;
        let result = contents.split_string_by_width(string, width);

        assert_eq!(result, vec!["Hello", ", wor", "ld!"]);
    }

    #[test]
    /// ASCII文字列と日本語文字列が混在する場合
    /// 日本語文字列は、2文字分の幅を占有する
    /// そのため、表示幅を考慮した行の切り出しをする
    fn test_split_string_by_width_ascii_and_japanese() {
        // インスタンスの値はなんでもいい
        let contents = Contents {
            original_contents: String::new(),
            cursor_x: 0,
            cursor_y: 0,
            width: 0,
            height: 0,
            contents: Vec::new(),
            x_start: 0,
            y_start: 0,
        };

        let string = "Hello, 世界!";
        let width = 5;
        let result = contents.split_string_by_width(string, width);
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
        // インスタンスの値はなんでもいい
        let contents = Contents {
            original_contents: String::new(),
            cursor_x: 0,
            cursor_y: 0,
            width: 0,
            height: 0,
            contents: Vec::new(),
            x_start: 0,
            y_start: 0,
        };

        // エスケープシーケンスが含まれる場合
        let string = "\x1b[31mHello, world!\x1b[0m";
        let width = 5;
        let result = contents.split_string_by_width(string, width);
        assert_eq!(result, vec!["\x1b[31mHello", ", wor", "ld!\x1b[0m"]);

        // マルチバイト文字列の場合
        let string = "\x1b[31mあい\x1b[0mう";
        let width = 5;
        let result = contents.split_string_by_width(string, width);
        // "\x1b[31m" は、表示幅が0だけど、次の"Hello"が表示幅を超えるので、"Hello"は次の行に移動する
        assert_eq!(result, vec!["\x1b[31mあい\x1b[0m", "う"]);
    }

    #[test]
    fn test_get_display_area() {
        let contents = Contents {
            original_contents: String::new(),
            cursor_x: 2,
            cursor_y: 3,
            width: 10,
            height: 5,
            contents: Vec::new(),
            x_start: 0,
            y_start: 0,
        };

        let (start_x, start_y, end_x, end_y) = contents.get_display_area();

        assert_eq!(start_x, 2);
        assert_eq!(start_y, 3);
        assert_eq!(end_x, 12);
        assert_eq!(end_y, 8);
    }
}
