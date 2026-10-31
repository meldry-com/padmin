pub use glory::prelude::*;
use std::hash::Hash;

pub fn each<T: Clone + 'static, K: Clone + Eq + Hash + 'static, W: Widget>(
    items: impl Into<Vary<Vec<T>>>,
    key: impl Fn(&T) -> K + 'static,
    mut render: impl FnMut(Cage<T>, Cage<usize>) -> W + 'static,
) -> impl Widget {
    glory::ui::each(items.into())
        .key(key)
        .children(move |row| render(row.value, row.index))
}
