# Alpine Linux
FROM alpine:3.19

# Install required dependencies
RUN apk add --no-cache \
    git \
    curl \
    jq \
    bash \
    ca-certificates

# Set working directory
WORKDIR /app

# Clone the vecnode repository
RUN git clone https://github.com/vecnode/vecnode.git . 

# Make scripts executable
RUN chmod +x scripts/download_all_repos.sh scripts/download_all_orgs.sh

# Create entrypoint script
RUN echo '#!/bin/bash\nset -e\necho "Starting downloads..."\nbash /app/scripts/download_all_repos.sh\nbash /app/scripts/download_all_orgs.sh\necho "Downloads complete!"\nbash' > /tmp/entrypoint.sh && \
    chmod +x /tmp/entrypoint.sh

# Run with entrypoint
ENTRYPOINT ["/bin/bash", "-c", "bash /app/scripts/download_all_repos.sh && bash /app/scripts/download_all_orgs.sh && bash"]
