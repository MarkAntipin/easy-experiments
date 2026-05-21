ARG NODE_VERSION=20

# ---------- Build ----------
FROM node:${NODE_VERSION}-alpine AS builder

WORKDIR /landing

COPY package.json package-lock.json ./
RUN --mount=type=cache,id=easy-experiments-landing-npm,target=/root/.npm \
    npm ci

COPY . ./

RUN npm run build

# ---------- Runtime ----------
FROM busybox:1-musl

COPY --from=builder /landing/dist /landing-dist

CMD ["true"]
