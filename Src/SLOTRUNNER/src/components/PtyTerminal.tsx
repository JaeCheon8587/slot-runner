import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { ptySpawn, ptyWrite, ptyResize, ptyKill, ptyPaste, ptyEnter, onPtyData } from "@/lib/pty";
import { hookPaths, installClaudeHooks } from "@/lib/hooks";
import { useAppStore } from "@/store/useAppStore";

// 슬롯 1개의 PTY(xterm) 렌더. mount 시 program(claude) 을 cwd 에서 spawn,
// 출력 stream→write, 입력→pty_write, resize→pty_resize. inject 가 있으면 준비 후 1회 주입.
// 슬라이스 3: 슬롯=claude 세션(ADR-002), forge-scope 슬래시 주입(ADR-003).
// 주입 지연은 config(injectDelayMs) — 휴리스틱, 후속 Hook SessionStart 로 대체.

export function PtyTerminal({
  id,
  program,
  args,
  cwd,
  inject,
}: {
  id: string;
  program?: string | null;
  args?: string[] | null;
  cwd?: string | null;
  inject?: string | null;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const term = new Terminal({
      fontSize: 12,
      fontFamily: "ui-monospace, Consolas, Menlo, monospace",
      theme: { background: "#191a1c", foreground: "#ccced3" },
      cursorBlink: true,
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(el);
    try {
      fit.fit();
    } catch {
      /* 초기 레이아웃 미정 시 무시 */
    }

    let disposed = false;
    let unlisten: (() => void) | null = null;
    let injected = false;

    onPtyData((pid, bytes) => {
      if (pid !== id) return;
      term.write(bytes);
      // 첫 출력 = 프로그램 기동 시작. 정착 지연 후 1회 주입(브래킷 페이스트 + Enter).
      if (inject && !injected) {
        injected = true;
        const delay = useAppStore.getState().injectDelayMs;
        setTimeout(() => {
          // 붙여넣기 후 충분히 정착시킨 뒤 Enter 제출. 동시 다중 세션 부하서 Enter 가
          // 묻히지 않도록 800ms (단일 세션 400ms 에서 상향).
          ptyPaste(id, inject)
            .then(() => setTimeout(() => ptyEnter(id), 800))
            .catch(() => {});
        }, delay);
      }
    })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch((e) => console.warn("[PtyTerminal] onPtyData 실패:", e));

    const doSpawn = () =>
      ptySpawn(id, { cwd, program, args }, term.cols, term.rows).catch((e) =>
        term.write(`\r\n[spawn 실패] ${e}\r\n`),
      );
    // 스텝 루프: target repo 에 훅 설치 후 spawn(Stop 이벤트 발화 → StageController 전이). 설치 실패해도 spawn.
    if (cwd && program === "claude") {
      hookPaths()
        .then((p) => installClaudeHooks(cwd, p.append_script))
        .catch((e) => term.write(`\r\n[훅 설치 경고] ${e}\r\n`))
        .finally(doSpawn);
    } else {
      doSpawn();
    }

    const onData = term.onData((d) => {
      // 사용자 직접 입력(수동 개입·취소, F006/ADR-005).
      ptyWrite(id, d).catch(() => {});
    });

    const ro = new ResizeObserver(() => {
      try {
        fit.fit();
        ptyResize(id, term.cols, term.rows).catch(() => {});
      } catch {
        /* 측정 불가 시 무시 */
      }
    });
    ro.observe(el);

    return () => {
      disposed = true;
      unlisten?.();
      onData.dispose();
      ro.disconnect();
      ptyKill(id).catch(() => {});
      term.dispose();
    };
  }, [id, program, cwd, inject, args]);

  return <div className="pty" ref={ref} />;
}
