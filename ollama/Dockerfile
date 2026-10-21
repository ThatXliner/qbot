FROM ollama/ollama:0.11.4

# Copy our wrapper script
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
# Ubuntu 24.04 trust fix for compromised eMudhra intermediates (USN-8944-1).
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && dpkg --compare-versions "$(dpkg-query -W -f='${Version}' ca-certificates)" ge 20260816ubuntu0.24.04.1 \
    && rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["/entrypoint.sh"]
