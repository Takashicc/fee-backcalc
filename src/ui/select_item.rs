use gpui::SharedString;
use gpui_component::select::SelectItem;

#[derive(Clone)]
pub(crate) struct EnumSelectItem<T: Clone> {
    label: SharedString,
    value: T,
}

impl<T: Clone> EnumSelectItem<T> {
    pub(crate) fn new(label: impl Into<SharedString>, value: T) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }

    pub(crate) fn value_ref(&self) -> &T {
        &self.value
    }
}

impl<T: Clone + 'static> SelectItem for EnumSelectItem<T> {
    type Value = T;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}
