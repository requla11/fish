// web — TypeScript frontend of the polyglot demo.
//
// Contract-first: the canonical topic list is owned by py-worker and imported
// directly across the project boundary. This relative import is the evidence
// `fish build` uses to infer web → py-worker and schedule accordingly.
import { EVENT_TOPICS } from "../../py-worker/contracts/topics.json";

type Topic = (typeof EVENT_TOPICS)["topics"][number];

const subscribed: Topic[] = [];

function subscribe(topic: string): void {
  if (!(EVENT_TOPICS.topics as readonly string[]).includes(topic)) {
    console.error(`✗ unknown topic: ${topic} (contract mismatch?)`);
    return;
  }
  subscribed.push(topic as Topic);
  console.log(`✓ subscribed: ${topic}`);
}

function main(): void {
  console.log("web client — allowed topics come from py-worker's contract:");
  for (const topic of EVENT_TOPICS.topics) {
    subscribe(topic);
  }
  subscribe("deploy.production"); // rejected by the contract on purpose

  const events = subscribed.map((topic) => ({
    id: `evt-${Math.random().toString(36).slice(2)}`,
    topic,
    created_at: new Date().toISOString(),
    payload: { source: "web" },
  }));
  console.log(`${events.length} well-formed event(s) ready to publish`);
}

main();
