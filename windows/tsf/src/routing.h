#pragma once

// Đánh dấu process đang có TSF ViFastKey active để hook biết phải pass-through.
// Hook đọc cùng named mapping; xem rust/apps/vfk-windows-replace/src/main.rs.
void PublishTsfActive();
void ClearTsfActive();
