// web-frontend — TypeScript frontend of the polyglot demo.
//
// Contract-first: the canonical topic list is owned by py-worker and imported
// directly across the project boundary. This relative import is the evidence
// `fish build` uses to infer web-frontend → py-worker and schedule accordingly.
// resolveJsonModule exposes each top-level JSON key as a typed named export.
import { topics } from "../../py-worker/contracts/topics.json";

type Topic = (typeof topics)[number];

interface ServiceResponse {
  message: string;
  timestamp: number;
}

const SERVICES = ["http://localhost:8080", "http://localhost:8081"];

const subscribed: Topic[] = [];

function subscribe(topic: string): void {
  if (!topics.includes(topic as (typeof topics)[number])) {
    console.error(`✗ unknown topic: ${topic} (contract mismatch?)`);
    return;
  }
  subscribed.push(topic as Topic);
  console.log(`✓ subscribed: ${topic}`);
}

function publishEvents(): void {
  const events = subscribed.map((topic) => ({
    id: `evt-${Math.random().toString(36).slice(2)}`,
    topic,
    created_at: new Date().toISOString(),
    payload: { source: "web-frontend" },
  }));
  console.log(`${events.length} well-formed TaskEvent(s) ready to publish`);
}

async function callAllServices(): Promise<ServiceResponse[]> {
  const responses: ServiceResponse[] = [];
  for (const service of SERVICES) {
    try {
      const response = await fetch(service);
      responses.push({ message: await response.text(), timestamp: Date.now() });
    } catch (error) {
      console.error(`Failed to call ${service}:`, error);
    }
  }
  return responses;
}

async function main(): Promise<void> {
  console.log("🌐 TypeScript Frontend starting...");
  console.log("Allowed topics come from py-worker's contract:");
  for (const topic of topics) {
    subscribe(topic);
  }
  subscribe("deploy.production"); // rejected by the contract on purpose
  publishEvents();

  const responses = await callAllServices();
  console.log("Service responses:", responses);
}

main().catch(console.error);
