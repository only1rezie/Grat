import { Worker } from "bullmq";
import { config } from "../config";

const worker = new Worker(
  "replay",
  async (job) => {
    console.log(`Processing replay job ${job.id}: ${job.data.txHash}`);
  },
  { connection: { url: config.redisUrl } }
);

export default worker;
