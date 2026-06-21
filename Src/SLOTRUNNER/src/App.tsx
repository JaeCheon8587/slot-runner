import { useEffect } from "react";
import { JobBridge } from "@/components/JobBridge";
import { StageController } from "@/components/StageController";
import { SlotPanel } from "@/components/SlotPanel";
import { EndOfRunModal } from "@/components/EndOfRunModal";
import { useAppStore } from "@/store/useAppStore";
import { loadConfig } from "@/lib/config";

// 슬라이스 7 UI: 슬롯 풀(좌, config 기반 N) + 공용 콘솔(우, 최우측). 슬라이스 6 EndOfRunModal.
export default function App() {
  const events = useAppStore((s) => s.consoleEvents);
  const slots = useAppStore((s) => s.slots);
  const queueLen = useAppStore((s) => s.queue.length);
  const clearConsole = useAppStore((s) => s.clearConsole);
  const applyConfig = useAppStore((s) => s.applyConfig);
  const addEvent = useAppStore((s) => s.addEvent);

  useEffect(() => {
    loadConfig()
      .then((cfg) => {
        applyConfig(cfg);
        addEvent("SYSTEM", `설정 적용 — 슬롯 ${cfg.slots} · 주입지연 ${cfg.inject_delay_ms}ms · REST :${cfg.rest_port}`);
      })
      .catch(() => {
        // 설정 로드 실패 시 기본값 유지.
      });
  }, [applyConfig, addEvent]);

  return (
    <div className="app">
      <JobBridge />
      <StageController />
      <EndOfRunModal />
      <header className="bar">
        <span className="brand">SlotRunner</span>
        <span className="sub">
          슬롯 풀({slots.length}) + 큐({queueLen})
        </span>
        <button className="btn" onClick={clearConsole}>
          콘솔 비우기
        </button>
      </header>
      <div className="body">
        <div className="slots">
          {slots.map((slot) => (
            <SlotPanel key={slot.id} id={slot.id} />
          ))}
        </div>
        <main className="console">
          {events.map((e) => (
            <div key={e.id} className="line">
              <span className="ts">{new Date(e.ts).toLocaleTimeString()}</span>
              <span className={`tag tag-${e.source.toLowerCase()}`}>[{e.source}]</span>
              <span className="msg">{e.message}</span>
            </div>
          ))}
        </main>
      </div>
    </div>
  );
}
