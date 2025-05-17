import type * as React from "react";
import type { Client, Player, SteamID, Uuid } from "../main";

export const Main: (
  props: {
    clients: Record<Uuid, Client>,
    players: Record<SteamID, Player>,
  }
) => React.ReactElement = (props) => {
  const playerCount = Object.keys(props.players).length;
  const clientEntries = Object.entries(props.clients);

  return (
    <div style={{ margin: "0", backgroundColor: "#0d1117", color: "#c9d1d9", fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif", lineHeight: "1.6" }}>
      <header style={{ backgroundColor: "#161b22", color: "#c9d1d9", padding: "16px 24px", fontSize: "18px", fontWeight: 600, borderBottom: "1px solid #30363d" }}>
        rustctl
      </header>

      <main style={{ maxWidth: "960px", margin: "32px auto", padding: "0 24px" }}>
        <div style={{ backgroundColor: "#161b22", border: "1px solid #30363d", borderRadius: "6px", padding: "24px" }}>

          <h2 style={{ fontSize: "18px", fontWeight: 600, borderBottom: "1px solid #30363d", paddingBottom: "6px", marginTop: "24px", marginBottom: "16px" }}>
            Player Statistics
          </h2>
          <div style={{ backgroundColor: "#0e141b", border: "2px dashed #30363d", borderRadius: "6px", height: "400px", display: "flex", alignItems: "center", justifyContent: "center", color: "#8b949e", marginBottom: "16px" }}>
            Player stats and recent event feed placeholder
          </div>

          <h2 style={{ fontSize: "18px", fontWeight: 600, borderBottom: "1px solid #30363d", paddingBottom: "6px", marginTop: "24px", marginBottom: "16px" }}>
            Real-Time World Map
          </h2>
          <div style={{ backgroundColor: "#0e141b", border: "2px dashed #30363d", borderRadius: "6px", height: "400px", display: "flex", alignItems: "center", justifyContent: "center", color: "#8b949e", marginBottom: "16px" }}>
            Real-time map rendering placeholder
          </div>

          <h2 style={{ fontSize: "18px", fontWeight: 600, borderBottom: "1px solid #30363d", paddingBottom: "6px", marginTop: "24px", marginBottom: "16px" }}>
            Players ({playerCount})
          </h2>
          <table style={{ width: "100%", borderCollapse: "collapse", backgroundColor: "#161b22", border: "1px solid #30363d", borderRadius: "6px", overflow: "hidden", marginBottom: "16px" }}>
            <thead>
              <tr>
                <th style={{ backgroundColor: "#1f242d", color: "#c9d1d9", fontWeight: 600, padding: "12px 16px", textAlign: "left", borderBottom: "1px solid #30363d" }}>Player Name</th>
                <th style={{ backgroundColor: "#1f242d", color: "#c9d1d9", fontWeight: 600, padding: "12px 16px", textAlign: "left", borderBottom: "1px solid #30363d" }}>Country</th>
                <th style={{ backgroundColor: "#1f242d", color: "#c9d1d9", fontWeight: 600, padding: "12px 16px", textAlign: "left", borderBottom: "1px solid #30363d" }}>Steam ID</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td style={{ padding: "12px 16px", textAlign: "left", borderBottom: "1px solid #30363d" }}>Placeholder</td>
                <td style={{ padding: "12px 16px", textAlign: "left", borderBottom: "1px solid #30363d" }}>
                  <img src="#" alt="" style={{ width: "24px", height: "16px", verticalAlign: "middle", marginRight: "8px" }} />N/A
                </td>
                <td style={{ padding: "12px 16px", textAlign: "left", borderBottom: "1px solid #30363d", color: "#8b949e" }}>00000000000000000</td>
              </tr>
            </tbody>
          </table>

          <h2 style={{ fontSize: "18px", fontWeight: 600, borderBottom: "1px solid #30363d", paddingBottom: "6px", marginTop: "24px", marginBottom: "16px" }}>
            Web Portal Clients ({clientEntries.length})
          </h2>
          <table style={{ width: "100%", borderCollapse: "collapse", backgroundColor: "#161b22", border: "1px solid #30363d", borderRadius: "6px", overflow: "hidden", marginBottom: "16px" }}>
            <thead>
              <tr>
                <th style={{ backgroundColor: "#1f242d", color: "#c9d1d9", fontWeight: 600, padding: "12px 16px", textAlign: "left", borderBottom: "1px solid #30363d" }}>Identity</th>
                <th style={{ backgroundColor: "#1f242d", color: "#c9d1d9", fontWeight: 600, padding: "12px 16px", textAlign: "left", borderBottom: "1px solid #30363d" }}>Connected At</th>
              </tr>
            </thead>
            <tbody>
              {clientEntries.map(([uuid, client]) => (
                <tr key={uuid}>
                  <td style={{ padding: "12px 16px", textAlign: "left", borderBottom: "1px solid #30363d", color: "#8b949e" }}>{client.identity}</td>
                  <td style={{ padding: "12px 16px", textAlign: "left", borderBottom: "1px solid #30363d" }}>{formatDate(new Date(client.connected_at))}</td>
                </tr>
              ))}
            </tbody>
          </table>

          <h2 style={{ fontSize: "18px", fontWeight: 600, borderBottom: "1px solid #30363d", paddingBottom: "6px", marginTop: "24px", marginBottom: "16px" }}>
            Dashboard
          </h2>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: "16px", marginBottom: "24px" }}>
            <button style={{ backgroundColor: "#21262d", color: "#c9d1d9", border: "1px solid #30363d", padding: "12px", fontSize: "16px", fontWeight: 500, borderRadius: "6px", textAlign: "center" }}>
              Stop
            </button>
            <button style={{ backgroundColor: "#21262d", color: "#c9d1d9", border: "1px solid #30363d", padding: "12px", fontSize: "16px", fontWeight: 500, borderRadius: "6px", textAlign: "center" }}>
              Install / Update and Start
            </button>
          </div>

          <h2 style={{ fontSize: "18px", fontWeight: 600, borderBottom: "1px solid #30363d", paddingBottom: "6px", marginTop: "24px", marginBottom: "16px" }}>
            System Resource Monitor
          </h2>
          <div style={{ backgroundColor: "#0e141b", border: "2px dashed #30363d", borderRadius: "6px", height: "400px", display: "flex", alignItems: "center", justifyContent: "center", color: "#8b949e", marginBottom: "16px" }}>
            CPU / Memory / I/O usage graphs placeholder
          </div>

        </div>
      </main>
    </div>
  );
};

function formatDate(date: Date): string {
  return date.toLocaleString(undefined, {
    dateStyle: "short",
    timeStyle: "short"
  });
}
