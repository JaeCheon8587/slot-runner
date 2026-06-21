import { useEffect } from "react";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { listenHookEvent } from "@/lib/hooks";
import { useAppStore } from "@/store/useAppStore";

// 데스크톱 토스트 브리지 (F008, sidabari4loop HookBridge 참고). App 루트 1회 마운트.
// 백그라운드로 앱을 띄워둔 운영자에게 "개입 필요" 시점을 알린다 — 자동 복구는 하지 않는다
// (스톨 시 자동 넛지 없음. 운영자가 슬롯 PTY 직접 입력(F006)으로 결정 — 선택권 보존).
//   · Notification 훅(claude 입력 대기·권한 프롬프트 등) → 토스트 "입력 대기/알림"
//   · 슬롯 완료/실패(outcome 발생) → 토스트 "완료/실패"
// 콘솔에도 미러링한다.

// 권한은 모듈 레벨 1회만 요청. 거부되면 이후 토스트 생략.
let permissionPromise: Promise<boolean> | null = null;
function ensurePermission(): Promise<boolean> {
  if (!permissionPromise) {
    permissionPromise = (async () => {
      try {
        if (await isPermissionGranted()) return true;
        return (await requestPermission()) === "granted";
      } catch {
        return false;
      }
    })();
  }
  return permissionPromise;
}

async function toast(title: string, body: string) {
  if (!(await ensurePermission())) return;
  try {
    sendNotification({ title, body });
  } catch {
    // 토스트 실패는 비차단 — 콘솔 미러로 충분.
  }
}

export function NotificationBridge() {
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    // 1) Notification 훅 → 토스트 + 콘솔. (입력 대기·권한 등 = 사람 개입 신호)
    listenHookEvent((e) => {
      if (e.kind !== "notification" || !e.panel_id) return;
      const ntype =
        typeof e.payload.notification_type === "string" ? e.payload.notification_type : "알림";
      const msg = typeof e.payload.message === "string" ? e.payload.message : "";
      const body = msg ? `${ntype} — ${msg.slice(0, 120)}` : ntype;
      useAppStore.getState().addEvent("HOOK", `${e.panel_id} notification: ${body}`);
      void toast(`SlotRunner — ${e.panel_id}`, body);
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch((err) => console.warn("[NotificationBridge] listen 실패:", err));

    // 2) 슬롯 완료/실패(outcome 새로 발생) → 토스트. 이전 outcome 과 비교해 전이만 알림.
    const prev = new Map<string, string | null>();
    useAppStore.getState().slots.forEach((s) => prev.set(s.id, s.outcome ? s.outcome.kind : null));
    const unsub = useAppStore.subscribe((state) => {
      for (const s of state.slots) {
        const cur = s.outcome ? s.outcome.kind : null;
        if (cur && cur !== prev.get(s.id)) {
          const label = cur === "done" ? "완료" : "실패";
          void toast(`SlotRunner — ${s.id}`, `${label}${s.outcome?.reason ? ` — ${s.outcome.reason}` : ""}`);
        }
        prev.set(s.id, cur);
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
      unsub();
    };
  }, []);

  return null;
}
