#!/usr/bin/env node
// SlotRunner Hook 이벤트 수집(일방향). claude 훅이 `node append-event.js <EventName>` 로 호출.
// stdin 의 hook payload(JSON)에 _slotrunner 메타(panel_id, event name)를 붙여 events.jsonl 에 1줄 append.
// 외부 텍스트를 명령으로 실행하지 않는다 — append 만. 항상 exit 0 (Stop 훅 강제연속 금지).
const fs = require("fs");
const path = require("path");

const eventName = process.argv[2] || "Unknown";
const baseDir = path.join(__dirname, ".."); // scripts/ -> slotrunner-hooks/
const eventsPath = path.join(baseDir, "events.jsonl");

let input = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (c) => (input += c));
process.stdin.on("end", () => {
  let payload = {};
  try {
    payload = input.trim() ? JSON.parse(input) : {};
  } catch {
    payload = {};
  }
  payload._slotrunner = {
    panel_id: process.env.SLOTRUNNER_PANEL_ID || null,
    hook_event_name_arg: eventName,
  };
  try {
    fs.appendFileSync(eventsPath, JSON.stringify(payload) + "\n");
  } catch {
    // 쓰기 실패는 무시(훅은 비차단·exit 0).
  }
  process.exit(0);
});
