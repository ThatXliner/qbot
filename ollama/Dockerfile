FROM ollama/ollama:0.11.4

# Copy our wrapper script
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
RUN apt-get update && apt-get install -y curl

ENTRYPOINT ["/entrypoint.sh"]
