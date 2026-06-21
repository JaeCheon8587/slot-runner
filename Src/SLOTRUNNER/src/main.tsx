import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";

// StrictMode 미사용 — dev 이중 effect 실행이 PTY 중복 spawn("pty 이미 존재")을 유발하기 때문.
// (PTY 생명주기는 멱등 spawn + 명시 kill 로 관리. 슬롯=세션 1:1, ADR-002.)
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(<App />);
