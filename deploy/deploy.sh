# Copy this onto the lightsail instance

docker run -it --rm -e DISCORD_TOKEN=$DISCORD_TOKEN -e GEMINI_API_KEY=$GEMINI_API_KEY ghcr.io/thatxliner/qbot:main
