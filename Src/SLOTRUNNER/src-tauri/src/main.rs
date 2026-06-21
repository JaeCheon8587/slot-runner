// Windows 릴리즈에서 콘솔 창 숨김(데스크톱앱). 디버그는 콘솔 유지(로그 가시).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    slotrunner_lib::run()
}
