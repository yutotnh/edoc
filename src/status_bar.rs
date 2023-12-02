use std::io::stdout;

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Print, ResetColor},
    QueueableCommand,
};

/// ステータスバーの項目
pub struct StatusBarItem {
    /// 項目名
    name: String,
    /// 項目の値
    value: String,
}

impl StatusBarItem {
    /// StatusBarItemを作成する
    pub fn new(name: String, mut value: String) -> Self {
        // 表示できるのは一行のみなので、改行を全て" "(スペース)に置換する
        value = value.replace('\n', " ");

        Self { name, value }
    }
}

/// ステータスバー
pub struct StatusBar {
    /// ステータスバーの項目
    items: Vec<StatusBarItem>,
    /// 幅
    pub width: u16,
    /// 高さ
    pub height: u16,
    /// 開始位置(X座標)
    pub x_start: u16,
    /// 開始位置(Y座標)
    pub y_start: u16,
}

impl StatusBar {
    pub fn new(width: u16, height: u16, x_start: u16, y_start: u16) -> Self {
        Self {
            items: vec![],
            width,
            height,
            x_start,
            y_start,
        }
    }

    pub fn add_item(&mut self, item: StatusBarItem) {
        // 過去に同じ名前の項目がある場合は、上書きする
        for i in 0..self.items.len() {
            if self.items[i].name == item.name {
                self.items[i] = item;
                return;
            }
        }

        self.items.push(item);
    }

    pub fn print(&self) {
        stdout().queue(MoveTo(0, self.y_start)).unwrap();

        // ステータスバーの文字色と背景色を反転する
        queue!(stdout(), Print("\x1b[7m")).unwrap();

        // 1行すべてを背景色で塗りつぶす
        queue!(stdout(), Print(" ".repeat(self.width as usize))).unwrap();

        // カーソルが移動したので、行の先頭に移動する
        stdout().queue(MoveTo(0, self.y_start)).unwrap();

        // ステータスバーの項目を表示する
        // 項目の間には" "を表示する
        self.items.iter().for_each(|item| {
            queue!(stdout(), Print(item.value.as_str())).unwrap();

            // 最後の項目以外は" "を表示する
            if item.name != self.items.last().unwrap().name.as_str() {
                queue!(stdout(), Print(" ")).unwrap();
            }
        });

        // ステータスバーの背景色をリセットする
        queue!(stdout(), ResetColor).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_bar() {
        let mut status_bar = StatusBar::new(10, 1, 0, 0);

        // 項目を追加することができるか確認する
        let item1 = StatusBarItem::new("item1".to_string(), "value1".to_string());
        let item2 = StatusBarItem::new("item2".to_string(), "value2".to_string());

        status_bar.add_item(item1);
        status_bar.add_item(item2);

        assert_eq!(status_bar.items.len(), 2);
        assert_eq!(status_bar.items[0].name, "item1");
        assert_eq!(status_bar.items[0].value, "value1");
        assert_eq!(status_bar.items[1].name, "item2");
        assert_eq!(status_bar.items[1].value, "value2");

        // 過去に同じ名前の項目がある場合は上書きするか確認する
        let item3 = StatusBarItem::new("item1".to_string(), "value3".to_string());
        status_bar.add_item(item3);

        assert_eq!(status_bar.items.len(), 2);
        assert_eq!(status_bar.items[0].name, "item1");
        assert_eq!(status_bar.items[0].value, "value3");
        assert_eq!(status_bar.items[1].name, "item2");
        assert_eq!(status_bar.items[1].value, "value2");

        // 表示できるのは一行のみなので、改行を全て" "(スペース)に置換していることを確認する
        let item4 = StatusBarItem::new("item4".to_string(), "value4\nvalue4".to_string());
        status_bar.add_item(item4);

        assert_eq!(status_bar.items.len(), 3);
        assert_eq!(status_bar.items[0].name, "item1");
        assert_eq!(status_bar.items[0].value, "value3");
        assert_eq!(status_bar.items[1].name, "item2");
        assert_eq!(status_bar.items[1].value, "value2");
        assert_eq!(status_bar.items[2].name, "item4");
        assert_eq!(status_bar.items[2].value, "value4 value4");
    }
}
