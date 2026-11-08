use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VectorIcon {
    Play,
    Grid,
    Mods,
    Coffee,
    Gear,
    Pickaxe,
    Folder,
    Download,
    Stop,
    Refresh,
    Check,
}

pub fn paint_vector_icon(painter: &egui::Painter, icon: VectorIcon, rect: Rect, color: Color32) {
    let center = rect.center();
    let w = rect.width();
    let h = rect.height();
    let s = w.min(h);

    match icon {
        VectorIcon::Play => {
            // Triangle pointing right
            let half = s * 0.40;
            let p1 = Pos2::new(center.x - half * 0.7, center.y - half);
            let p2 = Pos2::new(center.x + half * 0.9, center.y);
            let p3 = Pos2::new(center.x - half * 0.7, center.y + half);
            painter.add(egui::Shape::convex_polygon(vec![p1, p2, p3], color, Stroke::NONE));
        }
        VectorIcon::Grid => {
            // 2x2 grid of small rounded rectangles
            let block = s * 0.34;
            let gap = s * 0.12;
            let r = egui::Rounding::same(block * 0.25);
            // Top-left
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(center.x - gap / 2.0 - block, center.y - gap / 2.0 - block),
                    Vec2::splat(block),
                ),
                r,
                color,
            );
            // Top-right
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(center.x + gap / 2.0, center.y - gap / 2.0 - block),
                    Vec2::splat(block),
                ),
                r,
                color,
            );
            // Bottom-left
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(center.x - gap / 2.0 - block, center.y + gap / 2.0),
                    Vec2::splat(block),
                ),
                r,
                color,
            );
            // Bottom-right
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(center.x + gap / 2.0, center.y + gap / 2.0),
                    Vec2::splat(block),
                ),
                r,
                color,
            );
        }
        VectorIcon::Mods => {
            // Package / puzzle box
            let box_w = s * 0.72;
            let box_h = s * 0.68;
            let box_rect = Rect::from_center_size(center, Vec2::new(box_w, box_h));
            painter.rect_stroke(box_rect, egui::Rounding::same(2.5), Stroke::new(1.8, color));
            // Horizontal line
            painter.line_segment(
                [
                    Pos2::new(box_rect.min.x, box_rect.min.y + box_h * 0.35),
                    Pos2::new(box_rect.max.x, box_rect.min.y + box_h * 0.35),
                ],
                Stroke::new(1.8, color),
            );
            // Vertical seam
            painter.line_segment(
                [
                    Pos2::new(center.x, box_rect.min.y + box_h * 0.35),
                    Pos2::new(center.x, box_rect.max.y),
                ],
                Stroke::new(1.8, color),
            );
        }
        VectorIcon::Coffee => {
            // Coffee mug + steam
            let mug_w = s * 0.54;
            let mug_h = s * 0.44;
            let mug_rect = Rect::from_min_size(
                Pos2::new(center.x - mug_w * 0.60, center.y - mug_h * 0.15),
                Vec2::new(mug_w, mug_h),
            );
            painter.rect_filled(
                mug_rect,
                egui::Rounding {
                    nw: 1.0,
                    ne: 1.0,
                    sw: 4.0,
                    se: 4.0,
                },
                color,
            );
            // Handle
            let handle_center = Pos2::new(mug_rect.max.x + 2.0, center.y + 1.0);
            painter.circle_stroke(handle_center, s * 0.15, Stroke::new(1.6, color));
            // Steam lines
            painter.line_segment(
                [
                    Pos2::new(center.x - mug_w * 0.28, mug_rect.min.y - 2.0),
                    Pos2::new(center.x - mug_w * 0.28, mug_rect.min.y - 6.0),
                ],
                Stroke::new(1.6, color),
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + mug_w * 0.05, mug_rect.min.y - 2.0),
                    Pos2::new(center.x + mug_w * 0.05, mug_rect.min.y - 7.0),
                ],
                Stroke::new(1.6, color),
            );
        }
        VectorIcon::Gear => {
            // Gear: outer circle + 4 tooth lines
            let radius = s * 0.30;
            painter.circle_stroke(center, radius, Stroke::new(2.4, color));
            for angle in [
                0.0_f32,
                std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_2,
                3.0 * std::f32::consts::FRAC_PI_4,
            ] {
                let dx = angle.cos() * (radius + s * 0.12);
                let dy = angle.sin() * (radius + s * 0.12);
                painter.line_segment(
                    [
                        Pos2::new(center.x - dx, center.y - dy),
                        Pos2::new(center.x + dx, center.y + dy),
                    ],
                    Stroke::new(2.4, color),
                );
            }
            painter.circle_filled(center, radius * 0.40, color);
        }
        VectorIcon::Pickaxe => {
            // Minecraft Pickaxe: head curve + diagonal handle
            let handle_start = Pos2::new(center.x - s * 0.32, center.y + s * 0.32);
            let handle_end = Pos2::new(center.x + s * 0.18, center.y - s * 0.18);
            painter.line_segment([handle_start, handle_end], Stroke::new(2.5, color));
            // Head
            let h1 = Pos2::new(center.x - s * 0.12, center.y - s * 0.34);
            let h2 = Pos2::new(center.x + s * 0.20, center.y - s * 0.20);
            let h3 = Pos2::new(center.x + s * 0.34, center.y + s * 0.12);
            painter.line_segment([h1, h2], Stroke::new(3.5, color));
            painter.line_segment([h2, h3], Stroke::new(3.5, color));
        }
        VectorIcon::Folder => {
            // Folder
            let f_w = s * 0.72;
            let f_h = s * 0.52;
            let f_rect = Rect::from_center_size(center + Vec2::new(0.0, 1.0), Vec2::new(f_w, f_h));
            painter.rect_stroke(f_rect, egui::Rounding::same(2.0), Stroke::new(1.8, color));
            let tab_rect = Rect::from_min_size(
                Pos2::new(f_rect.min.x, f_rect.min.y - 3.5),
                Vec2::new(f_w * 0.45, 4.0),
            );
            painter.rect_filled(tab_rect, egui::Rounding::same(1.5), color);
        }
        VectorIcon::Download => {
            // Down arrow into tray
            let arr_x = center.x;
            painter.line_segment(
                [
                    Pos2::new(arr_x, center.y - s * 0.35),
                    Pos2::new(arr_x, center.y + s * 0.15),
                ],
                Stroke::new(2.2, color),
            );
            painter.line_segment(
                [
                    Pos2::new(arr_x - s * 0.22, center.y - s * 0.05),
                    Pos2::new(arr_x, center.y + s * 0.18),
                ],
                Stroke::new(2.2, color),
            );
            painter.line_segment(
                [
                    Pos2::new(arr_x + s * 0.22, center.y - s * 0.05),
                    Pos2::new(arr_x, center.y + s * 0.18),
                ],
                Stroke::new(2.2, color),
            );
            let tray_y = center.y + s * 0.30;
            painter.line_segment(
                [
                    Pos2::new(arr_x - s * 0.32, tray_y - s * 0.10),
                    Pos2::new(arr_x - s * 0.32, tray_y),
                ],
                Stroke::new(2.0, color),
            );
            painter.line_segment(
                [
                    Pos2::new(arr_x - s * 0.32, tray_y),
                    Pos2::new(arr_x + s * 0.32, tray_y),
                ],
                Stroke::new(2.0, color),
            );
            painter.line_segment(
                [
                    Pos2::new(arr_x + s * 0.32, tray_y),
                    Pos2::new(arr_x + s * 0.32, tray_y - s * 0.10),
                ],
                Stroke::new(2.0, color),
            );
        }
        VectorIcon::Stop => {
            let stop_rect = Rect::from_center_size(center, Vec2::splat(s * 0.55));
            painter.rect_filled(stop_rect, egui::Rounding::same(2.5), color);
        }
        VectorIcon::Refresh => {
            painter.circle_stroke(center, s * 0.30, Stroke::new(2.0, color));
            let arrow_tip = Pos2::new(center.x + s * 0.30, center.y);
            painter.line_segment(
                [arrow_tip, Pos2::new(arrow_tip.x + 3.0, arrow_tip.y - 3.0)],
                Stroke::new(2.0, color),
            );
        }
        VectorIcon::Check => {
            let p1 = Pos2::new(center.x - s * 0.30, center.y);
            let p2 = Pos2::new(center.x - s * 0.08, center.y + s * 0.22);
            let p3 = Pos2::new(center.x + s * 0.30, center.y - s * 0.22);
            painter.line_segment([p1, p2], Stroke::new(2.2, color));
            painter.line_segment([p2, p3], Stroke::new(2.2, color));
        }
    }
}
