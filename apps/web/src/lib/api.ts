const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3001";

export async function requestReplay(txHash: string, network: string) {
  const res = await fetch(`${API_BASE}/api/replay`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ txHash, network }),
  });
  return res.json();
}

export async function getHealth() {
  const res = await fetch(`${API_BASE}/api/health`);
  return res.json();
}
