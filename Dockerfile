FROM node:24-alpine AS base
RUN npm install -g pnpm@11

FROM base AS development-dependencies-env
COPY . /app
WORKDIR /app
RUN pnpm install --frozen-lockfile

FROM base AS production-dependencies-env
COPY ./package.json ./pnpm-lock.yaml ./pnpm-workspace.yaml /app/
COPY ./apps/web/package.json /app/apps/web/package.json
WORKDIR /app
RUN pnpm install --frozen-lockfile --prod

FROM base AS build-env
COPY . /app/
COPY --from=development-dependencies-env /app/node_modules /app/node_modules
WORKDIR /app
RUN pnpm run build

FROM node:24-alpine
RUN apk add --no-cache git git-lfs && git lfs install
COPY --from=production-dependencies-env /app/ /app/
COPY --from=build-env /app/apps/web/build /app/apps/web/build
WORKDIR /app/apps/web
ENV NODE_ENV=production
CMD ["node", "node_modules/@react-router/serve/bin.cjs", "./build/server/index.js"]
