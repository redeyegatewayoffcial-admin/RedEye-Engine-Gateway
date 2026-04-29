# Build Stage
FROM node:lts-alpine as build

WORKDIR /app

# Copy package files
COPY redeye_dashboard/package*.json ./

# Install dependencies
RUN npm ci

# Copy the rest of the application
COPY redeye_dashboard/ ./

# Build the application
RUN npm run build

# Production Stage
FROM nginx:stable-alpine

# Copy built assets from build stage
COPY --from=build /app/dist /usr/share/nginx/html

# Copy custom nginx config if needed (optional)
# COPY redeye_dashboard/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
