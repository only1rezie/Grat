import type { FastifyInstance } from "fastify";

export async function sessionRoutes(app: FastifyInstance) {
  app.get("/session/:sessionId", async (request, reply) => {
    return { status: "not_found" };
  });
}
