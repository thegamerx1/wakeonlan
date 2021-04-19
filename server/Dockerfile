FROM node:14

ENV NODE_ENV=production
WORKDIR /usr/src/app
COPY package*.json ./
RUN yarn install
RUN apt update && apt install net-tools
COPY . .

EXPOSE 80
CMD [ "yarn", "start" ]