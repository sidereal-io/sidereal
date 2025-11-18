FROM node:24-alpine

WORKDIR /app

RUN corepack enable && corepack prepare pnpm@9.0.0 --activate

COPY pnpm-workspace.yaml package.json pnpm-lock.yaml ./
COPY packages ./packages
COPY apps/frontend ./apps/frontend

RUN pnpm install --frozen-lockfile

WORKDIR /app/apps/frontend

CMD ["pnpm", "dev"]
