#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RectI {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PointI {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CardLayout {
    pub global_index: usize,
    pub card: RectI,
    pub preview: RectI,
    pub title: RectI,
    pub close: RectI,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowGridLayout {
    pub bounds: RectI,
    pub surface: RectI,
    pub header: RectI,
    pub cards: Vec<CardLayout>,
    pub page_start: usize,
    pub total_items: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridHitTarget {
    Card(usize),
    CloseButton(usize),
}

const PAGE_SIZE: usize = 10;
const MAX_COLUMNS: usize = 5;
const CARD_WIDTH: i32 = 256;
const PREVIEW_HEIGHT: i32 = 144;
const TITLE_HEIGHT: i32 = 28;
const HEADER_HEIGHT: i32 = 40;
const CARD_PADDING: i32 = 8;
const OUTER_PADDING: i32 = 14;
const GAP: i32 = 10;
const CLOSE_SIZE: i32 = 20;
const SHADOW_MARGIN: i32 = 12;
const WORK_AREA_PERCENT: i32 = 90;

impl RectI {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(self, point: PointI) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x + self.width
            && point.y < self.y + self.height
    }

    pub fn contains_rect(self, other: Self) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.width <= self.x + self.width
            && other.y + other.height <= self.y + self.height
    }

    pub fn center(self) -> PointI {
        PointI {
            x: self.x + self.width / 2,
            y: self.y + self.height / 2,
        }
    }
}

impl WindowGridLayout {
    pub fn new(work_area: RectI, dpi_scale: f64, total_items: usize, selected: usize) -> Self {
        let selected = selected.min(total_items.saturating_sub(1));
        let page_start = selected / PAGE_SIZE * PAGE_SIZE;
        let visible_items = total_items.saturating_sub(page_start).min(PAGE_SIZE);
        let rows = balanced_rows(visible_items);

        let (nominal_columns, nominal_rows) = if total_items > PAGE_SIZE {
            (MAX_COLUMNS, 2)
        } else {
            (rows.iter().copied().max().unwrap_or(0), rows.len())
        };
        let nominal_width = nominal_surface_width(nominal_columns) + SHADOW_MARGIN * 2;
        let nominal_height = nominal_surface_height(nominal_rows) + SHADOW_MARGIN * 2;
        let width_limit = work_area.width * WORK_AREA_PERCENT / 100;
        let height_limit = work_area.height * WORK_AREA_PERCENT / 100;
        let fit_scale = (width_limit as f64 / nominal_width as f64)
            .min(height_limit as f64 / nominal_height as f64);
        let mut scale = dpi_scale.min(fit_scale).max(0.5);

        if nominal_width as f64 * scale > work_area.width as f64
            || nominal_height as f64 * scale > work_area.height as f64
        {
            scale = fit_scale;
        }

        let card_width = scaled(CARD_WIDTH, scale);
        let preview_height = scaled(PREVIEW_HEIGHT, scale);
        let title_height = scaled(TITLE_HEIGHT, scale);
        let header_height = scaled(HEADER_HEIGHT, scale);
        let card_padding = scaled(CARD_PADDING, scale);
        let outer_padding = scaled(OUTER_PADDING, scale);
        let gap = scaled(GAP, scale);
        let close_size = scaled(CLOSE_SIZE, scale);
        let shadow_margin = scaled(SHADOW_MARGIN, scale);
        let card_height = preview_height + title_height + card_padding * 2;
        let surface_width = outer_padding * 2
            + card_width * nominal_columns as i32
            + gap * nominal_columns.saturating_sub(1) as i32;
        let surface_height = outer_padding * 2
            + header_height
            + card_height * nominal_rows as i32
            + gap * nominal_rows.saturating_sub(1) as i32;
        let bounds_width = surface_width + shadow_margin * 2;
        let bounds_height = surface_height + shadow_margin * 2;
        let bounds = RectI::new(
            work_area.x + (work_area.width - bounds_width) / 2,
            work_area.y + (work_area.height - bounds_height) / 2,
            bounds_width,
            bounds_height,
        );
        let surface = RectI::new(
            bounds.x + shadow_margin,
            bounds.y + shadow_margin,
            surface_width,
            surface_height,
        );
        let header = RectI::new(
            surface.x + outer_padding,
            surface.y + outer_padding,
            surface.width - outer_padding * 2,
            header_height,
        );
        let cards_top = header.y + header.height;
        let mut cards = Vec::with_capacity(visible_items);
        let mut local_index = 0;

        for (row_index, row_length) in rows.iter().copied().enumerate() {
            let row_width =
                card_width * row_length as i32 + gap * row_length.saturating_sub(1) as i32;
            let row_x = surface.x + (surface.width - row_width) / 2;
            let card_y = cards_top + row_index as i32 * (card_height + gap);

            for column in 0..row_length {
                let card = RectI::new(
                    row_x + column as i32 * (card_width + gap),
                    card_y,
                    card_width,
                    card_height,
                );
                let preview = RectI::new(
                    card.x + card_padding,
                    card.y + card_padding,
                    card.width - card_padding * 2,
                    preview_height,
                );
                let title = RectI::new(
                    preview.x,
                    preview.y + preview.height,
                    preview.width,
                    title_height,
                );
                let close = RectI::new(
                    card.x + card.width - card_padding - close_size,
                    card.y + card_padding,
                    close_size,
                    close_size,
                );

                cards.push(CardLayout {
                    global_index: page_start + local_index,
                    card,
                    preview,
                    title,
                    close,
                });
                local_index += 1;
            }
        }

        Self {
            bounds,
            surface,
            header,
            cards,
            page_start,
            total_items,
        }
    }

    pub fn row_lengths(&self) -> Vec<usize> {
        balanced_rows(self.cards.len())
    }

    pub fn hit_test(&self, point: PointI, quick_actions: bool) -> Option<GridHitTarget> {
        self.cards.iter().find_map(|card| {
            if quick_actions && card.close.contains(point) {
                Some(GridHitTarget::CloseButton(card.global_index))
            } else if card.card.contains(point) {
                Some(GridHitTarget::Card(card.global_index))
            } else {
                None
            }
        })
    }
}

pub fn advance_index(current: usize, len: usize, reverse: bool) -> usize {
    if len == 0 {
        0
    } else if reverse {
        (current + len - 1) % len
    } else {
        (current + 1) % len
    }
}

pub fn fit_aspect(source_width: i32, source_height: i32, frame: RectI) -> RectI {
    if source_width <= 0 || source_height <= 0 || frame.width <= 0 || frame.height <= 0 {
        return frame;
    }

    let (width, height) = if i64::from(source_width) * i64::from(frame.height)
        <= i64::from(source_height) * i64::from(frame.width)
    {
        (
            (i64::from(source_width) * i64::from(frame.height) / i64::from(source_height)) as i32,
            frame.height,
        )
    } else {
        (
            frame.width,
            (i64::from(source_height) * i64::from(frame.width) / i64::from(source_width)) as i32,
        )
    };

    RectI::new(
        frame.x + (frame.width - width) / 2,
        frame.y + (frame.height - height) / 2,
        width,
        height,
    )
}

fn balanced_rows(count: usize) -> Vec<usize> {
    if count <= MAX_COLUMNS {
        vec![count]
    } else {
        let first = count.div_ceil(2);
        vec![first, count - first]
    }
}

fn nominal_surface_width(columns: usize) -> i32 {
    OUTER_PADDING * 2 + CARD_WIDTH * columns as i32 + GAP * columns.saturating_sub(1) as i32
}

fn nominal_surface_height(rows: usize) -> i32 {
    let card_height = PREVIEW_HEIGHT + TITLE_HEIGHT + CARD_PADDING * 2;
    OUTER_PADDING * 2
        + HEADER_HEIGHT
        + card_height * rows as i32
        + GAP * rows.saturating_sub(1) as i32
}

fn scaled(metric: i32, scale: f64) -> i32 {
    (metric as f64 * scale).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK_AREA: RectI = RectI::new(0, 0, 1920, 1080);

    #[test]
    fn balances_six_through_ten_items() {
        let expected = [
            (6, vec![3, 3]),
            (7, vec![4, 3]),
            (8, vec![4, 4]),
            (9, vec![5, 4]),
            (10, vec![5, 5]),
        ];

        for (count, row_lengths) in expected {
            let layout = WindowGridLayout::new(WORK_AREA, 1.0, count, 1);
            assert_eq!(layout.row_lengths(), row_lengths);
            assert!(WORK_AREA.contains_rect(layout.bounds));
        }
    }

    #[test]
    fn pages_after_ten_items() {
        let first = WindowGridLayout::new(WORK_AREA, 1.0, 14, 9);
        let second = WindowGridLayout::new(WORK_AREA, 1.0, 14, 10);
        assert_eq!(first.page_start, 0);
        assert_eq!(first.cards.len(), 10);
        assert_eq!(second.page_start, 10);
        assert_eq!(
            second
                .cards
                .iter()
                .map(|c| c.global_index)
                .collect::<Vec<_>>(),
            vec![10, 11, 12, 13]
        );
        assert_eq!(first.cards[0].preview.width, second.cards[0].preview.width);
        assert_eq!(
            first.cards[0].preview.height,
            second.cards[0].preview.height
        );
    }

    #[test]
    fn stays_inside_work_area_at_supported_scales() {
        for dpi in [1.0, 1.25, 1.5, 2.0] {
            let layout = WindowGridLayout::new(WORK_AREA, dpi, 10, 1);
            assert!(WORK_AREA.contains_rect(layout.bounds));
        }
    }

    #[test]
    fn traverses_forward_reverse_and_wraps() {
        assert_eq!(advance_index(0, 8, false), 1);
        assert_eq!(advance_index(7, 8, false), 0);
        assert_eq!(advance_index(0, 8, true), 7);
        assert_eq!(advance_index(4, 8, true), 3);
        assert_eq!(advance_index(3, 0, false), 0);
    }

    #[test]
    fn aspect_fit_preserves_shape_and_centers() {
        let frame = RectI::new(10, 20, 300, 200);
        assert_eq!(fit_aspect(1600, 900, frame), RectI::new(10, 36, 300, 168));
        assert_eq!(fit_aspect(900, 1600, frame), RectI::new(104, 20, 112, 200));
        assert_eq!(fit_aspect(1000, 1000, frame), RectI::new(60, 20, 200, 200));
        assert_eq!(fit_aspect(0, 1000, frame), frame);
    }

    #[test]
    fn aspect_fit_avoids_overflowing_large_dimensions() {
        assert_eq!(
            fit_aspect(i32::MAX, i32::MAX, RectI::new(10, 20, 2, 2)),
            RectI::new(10, 20, 2, 2)
        );
        assert_eq!(
            fit_aspect(i32::MAX, i32::MAX, RectI::new(10, 20, 2, 3)),
            RectI::new(10, 20, 2, 2)
        );
    }

    #[test]
    fn hit_test_distinguishes_card_and_close_button() {
        let layout = WindowGridLayout::new(WORK_AREA, 1.0, 6, 1);
        let card = layout.cards[0];
        assert_eq!(
            layout.hit_test(card.close.center(), true),
            Some(GridHitTarget::CloseButton(0))
        );
        assert_eq!(
            layout.hit_test(card.preview.center(), true),
            Some(GridHitTarget::Card(0))
        );
        assert_eq!(
            layout.hit_test(card.close.center(), false),
            Some(GridHitTarget::Card(0))
        );
    }
}
