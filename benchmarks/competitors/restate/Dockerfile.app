FROM node:20-slim
WORKDIR /app
COPY package.json ./
RUN npm install
COPY workflow.ts serve.ts ./
CMD ["npx", "tsx", "serve.ts"]
