# Setup SSH on Ubuntu VM

**On the VM at 192.168.10.205:**

## Quick SSH Setup

```bash
# Install OpenSSH server
sudo apt update
sudo apt install -y openssh-server

# Start SSH service
sudo systemctl start ssh
sudo systemctl enable ssh

# Check it's running
sudo systemctl status ssh | head -5
# Should show: "active (running)"

# Check it's listening
ss -tlnp | grep :22
# Should show: LISTEN on port 22

# Allow SSH through firewall
sudo ufw allow ssh

# Get your username (for SSH login)
whoami
```

---

## Then From Your Dev Machine:

```bash
# Test SSH connection
ssh username@192.168.10.205
# Replace 'username' with what whoami showed

# If it asks about host key, type: yes

# You should be logged in!
```

---

## Now You Can Transfer Files Easily

```bash
# From your dev machine:
# Copy the entire project
rsync -av /home/greg/wayland/wrd-server-specs/ username@192.168.10.205:~/wayland-rdp/

# Or use scp
scp -r /home/greg/wayland/wrd-server-specs/* username@192.168.10.205:~/wayland-rdp/

# Or just copy specific files
scp /home/greg/wayland/wrd-server-specs/config.toml.example username@192.168.10.205:~/wayland-rdp/config.toml
```

---

## After SSH is Set Up

You can work much faster:

```bash
# SSH in
ssh username@192.168.10.205

# Navigate to project
cd ~/wayland-rdp

# Build
cargo build --release

# Run
./target/release/wrd-server -c config.toml -vv
```

**Much easier than VM console!**

