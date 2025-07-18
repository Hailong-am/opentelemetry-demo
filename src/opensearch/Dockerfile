# Copyright The OpenTelemetry Authors
# SPDX-License-Identifier: Apache-2.0

# Part 1: Build the skills plugin
FROM gradle:8.14-jdk21 AS builder

# Set working directory
WORKDIR /app

# Clone the source repository
RUN git clone https://github.com/Hailong-am/skills.git .

RUN git checkout log-pattens-analysis

# Build the plugin using Gradle
RUN ./gradlew clean assemble

# Part 2: Create the final OpenSearch image with plugin
FROM opensearchproject/opensearch:3.1.0

# Switch to root user to install plugin
USER root

# Copy the built plugin zip file from builder stage
COPY --from=builder /app/build/distributions/*.zip /tmp/skills-plugin.zip

# Remove existing skills plugin and install custom one
RUN /usr/share/opensearch/bin/opensearch-plugin remove opensearch-skills || true
RUN /usr/share/opensearch/bin/opensearch-plugin install file:///tmp/skills-plugin.zip --batch

# Clean up the temporary file
RUN rm -f /tmp/skills-plugin.zip
