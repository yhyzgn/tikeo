FROM oven/bun:1.3.13 AS builder

WORKDIR /app

COPY package.json bun.lock ./
RUN bun install --frozen-lockfile && rm -rf /tmp/*

COPY . .
RUN NODE_ENV=production bun run build

FROM nginx:alpine AS runtime

WORKDIR /usr/share/nginx/html
COPY --from=builder /app/dist .
COPY --from=builder /app/nginx/nginx.conf /etc/nginx/nginx.conf
COPY --from=builder /app/nginx/default.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
