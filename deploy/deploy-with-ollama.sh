# Copy this onto the lightsail instance
docker compose pull
docker compose up -d -e DISCORD_TOKEN=$DISCORD_TOKEN
