# PERFORMANCE REQUIREMENTS
**Document:** PERFORMANCE-REQUIREMENTS.md
**Version:** 1.0

## LATENCY TARGETS

### Input Latency
| Priority | Target | Maximum | Measurement Method |
|----------|--------|---------|-------------------|
| HIGH | < 30ms | < 50ms | Keypress to compositor event |

### Encoding Latency
| Resolution | Encoder | Target | Maximum |
|------------|---------|--------|---------|
| 1080p @ 30 FPS | VA-API | < 16ms | < 33ms |
| 1080p @ 30 FPS | OpenH264 | < 50ms | < 100ms |
| 4K @ 30 FPS | VA-API | < 33ms | < 66ms |

### End-to-End Latency
| Network | Target | Maximum |
|---------|--------|---------|
| LAN (< 1ms RTT) | < 50ms | < 100ms |
| WAN (< 50ms RTT) | < 150ms | < 300ms |

## THROUGHPUT TARGETS

### Frame Rate
- Minimum: 24 FPS (acceptable)
- Target: 30 FPS (smooth)
- Maximum: 60 FPS (optional)

### Bitrate
| Resolution | FPS | Target Bitrate | Maximum |
|------------|-----|----------------|---------|
| 1920x1080 | 30 | 4 Mbps | 8 Mbps |
| 1920x1080 | 60 | 8 Mbps | 16 Mbps |
| 3840x2160 | 30 | 12 Mbps | 24 Mbps |

## RESOURCE USAGE TARGETS

### CPU Usage
| State | VA-API | OpenH264 |
|-------|--------|----------|
| Idle | < 2% | < 2% |
| Active 1080p30 | < 10% | < 50% |
| Active 4K30 | < 20% | N/A |

### Memory Usage
| State | Target | Maximum |
|-------|--------|---------|
| Base | < 200 MB | < 300 MB |
| Per Connection | < 100 MB | < 200 MB |
| Total (10 connections) | < 1.5 GB | < 2.5 GB |

### GPU Usage (VA-API)
| Resolution | Target | Maximum |
|------------|--------|---------|
| 1080p30 | < 15% | < 30% |
| 4K30 | < 30% | < 60% |

### Network Bandwidth
| Scenario | Average | Peak |
|----------|---------|------|
| 1080p30 + Audio | 4 Mbps | 6 Mbps |
| Multi-monitor (2x1080p30) | 8 Mbps | 12 Mbps |

## SCALABILITY TARGETS

### Concurrent Connections
- Per Instance: 10 connections (default)
- Maximum: 50 connections (with adequate resources)

### Multi-Monitor
- Minimum: 2 monitors
- Target: 4 monitors
- Maximum: 8 monitors (if hardware supports)

## PERFORMANCE MEASUREMENT

### Tools
```bash
# Latency measurement
ping -c 100 server-ip

# Frame rate
# Monitor RDP client FPS counter

# CPU/Memory
top -p $(pgrep wrd-server)

# GPU
intel_gpu_top  # Intel
radeontop      # AMD

# Network
iftop
nethogs
```

### Benchmark Suite
```bash
# Run all benchmarks
cargo bench

# Specific benchmark
cargo bench encoding
```

## PERFORMANCE OPTIMIZATION

### 1. Enable Hardware Encoding
```toml
[video]
encoder = "vaapi"
```

### 2. Optimize Thread Pool
```toml
[performance]
encoder_threads = 4  # Match CPU cores
network_threads = 2
```

### 3. Enable Zero-Copy
```toml
[performance]
zero_copy = true
```

### 4. Tune PipeWire Latency
```bash
export PIPEWIRE_LATENCY=512/48000
```

### 5. Kernel Tuning
```bash
# Increase network buffers
sysctl -w net.core.rmem_max=134217728
sysctl -w net.core.wmem_max=134217728
```

## PERFORMANCE REGRESSION PREVENTION

### Continuous Benchmarking
- Run benchmarks on every commit
- Alert on > 10% regression
- Require investigation before merge

### Performance Tests in CI
```yaml
- name: Performance Tests
  run: |
    cargo bench --no-fail-fast
    python3 scripts/compare-bench.py baseline.json current.json
```

## END OF PERFORMANCE REQUIREMENTS
