#!/usr/bin/env bash

# ==============================================================================
# AAE App — Project Requirements Check & Installer
# ==============================================================================

# Exit immediately if a command exits with a non-zero status
# set -e

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Emojis/Symbols
CHECK="✔"
CROSS="✘"
WARN="⚠"
INFO="ℹ"

# OS Detection
OS_TYPE=$(uname -s)

# Function to print header
print_header() {
  echo -e "${BOLD}${BLUE}=================================================================${NC}"
  echo -e "${BOLD}${BLUE}        Armellini Air Express — Project Requirements            ${NC}"
  echo -e "${BOLD}${BLUE}=================================================================${NC}"
  echo -e "${BOLD}Current OS Detected:${NC} ${CYAN}${OS_TYPE}${NC}\n"
}

# Function to check version greater than or equal
check_semver_ge() {
  local ver1=$1
  local ver2=$2
  [ "$(printf '%s\n' "$ver1" "$ver2" | sort -V | head -n1)" = "$ver2" ]
}

# Global status tracking variables
MISSING_ESSENTIAL=0
MISSING_OPTIONAL=0

# Core tool checks
check_git() {
  echo -e "${BOLD}1. Git Version Control:${NC}"
  if command -v git >/dev/null 2>&1; then
    GIT_VER=$(git --version | awk '{print $3}')
    echo -e "   ${GREEN}${CHECK} Git is installed (v$GIT_VER)${NC}"
    return 0
  else
    echo -e "   ${RED}${CROSS} Git is NOT installed${NC}"
    MISSING_ESSENTIAL=$((MISSING_ESSENTIAL + 1))
    return 1
  fi
}

check_github() {
  echo -e "${BOLD}2. GitHub Connection:${NC}"
  local has_ssh=0
  local has_gh=0

  # Test SSH connection
  ssh_out=$(ssh -o ConnectTimeout=3 -o StrictHostKeyChecking=no -T git@github.com 2>&1)
  if echo "$ssh_out" | grep -qE "successfully authenticated|Hi "; then
    echo -e "   ${GREEN}${CHECK} GitHub SSH: Authenticated successfully${NC}"
    has_ssh=1
  else
    echo -e "   ${YELLOW}${WARN} GitHub SSH: Authentication failed or no keys found${NC}"
  fi

  # Test GitHub CLI
  if command -v gh >/dev/null 2>&1; then
    if gh auth status >/dev/null 2>&1; then
      echo -e "   ${GREEN}${CHECK} GitHub CLI: Logged in${NC}"
      has_gh=1
    else
      echo -e "   ${YELLOW}${WARN} GitHub CLI: Installed, but not logged in${NC}"
    fi
  else
    echo -e "   ${CYAN}${INFO} GitHub CLI: Not installed${NC}"
  fi

  if [ $has_ssh -eq 1 ] || [ $has_gh -eq 1 ]; then
    return 0
  else
    echo -e "   ${YELLOW}${WARN} Action Required: Add your SSH key to GitHub or run 'gh auth login'${NC}"
    return 1
  fi
}

check_node() {
  echo -e "${BOLD}3. Node.js Runtime:${NC}"
  if command -v node >/dev/null 2>&1; then
    NODE_VER=$(node -v | sed 's/v//')
    if check_semver_ge "$NODE_VER" "20.0.0"; then
      echo -e "   ${GREEN}${CHECK} Node.js is installed (v$NODE_VER)${NC}"
      return 0
    else
      echo -e "   ${YELLOW}${WARN} Node.js is installed (v$NODE_VER), but v20+ is required${NC}"
      MISSING_ESSENTIAL=$((MISSING_ESSENTIAL + 1))
      return 1
    fi
  else
    echo -e "   ${RED}${CROSS} Node.js is NOT installed${NC}"
    MISSING_ESSENTIAL=$((MISSING_ESSENTIAL + 1))
    return 2
  fi
}

check_npm() {
  echo -e "${BOLD}4. NPM Package Manager:${NC}"
  if command -v npm >/dev/null 2>&1; then
    NPM_VER=$(npm -v)
    echo -e "   ${GREEN}${CHECK} NPM is installed (v$NPM_VER)${NC}"
    return 0
  else
    echo -e "   ${RED}${CROSS} NPM is NOT installed${NC}"
    MISSING_ESSENTIAL=$((MISSING_ESSENTIAL + 1))
    return 1
  fi
}

check_firebase() {
  echo -e "${BOLD}5. Firebase CLI Tools:${NC}"
  if command -v firebase >/dev/null 2>&1; then
    FIREBASE_VER=$(firebase --version 2>/dev/null || echo "unknown")
    echo -e "   ${GREEN}${CHECK} Firebase CLI is installed (v$FIREBASE_VER)${NC}"
    return 0
  else
    echo -e "   ${RED}${CROSS} Firebase CLI is NOT installed${NC}"
    MISSING_ESSENTIAL=$((MISSING_ESSENTIAL + 1))
    return 1
  fi
}

check_gcloud() {
  echo -e "${BOLD}6. Google Cloud SDK:${NC}"
  if command -v gcloud >/dev/null 2>&1; then
    GCLOUD_VER=$(gcloud --version 2>/dev/null | head -n 1 | awk '{print $4}')
    echo -e "   ${GREEN}${CHECK} Google Cloud CLI is installed (v$GCLOUD_VER)${NC}"
    return 0
  else
    echo -e "   ${RED}${CROSS} Google Cloud CLI is NOT installed${NC}"
    MISSING_ESSENTIAL=$((MISSING_ESSENTIAL + 1))
    return 1
  fi
}

check_docker() {
  echo -e "${BOLD}7. Docker Container Engine (Optional):${NC}"
  if command -v docker >/dev/null 2>&1; then
    DOCKER_VER=$(docker --version | awk '{print $3}' | sed 's/,//')
    if docker info >/dev/null 2>&1; then
      echo -e "   ${GREEN}${CHECK} Docker is installed and running (v$DOCKER_VER)${NC}"
      return 0
    else
      echo -e "   ${YELLOW}${WARN} Docker is installed (v$DOCKER_VER), but the daemon is not running${NC}"
      MISSING_OPTIONAL=$((MISSING_OPTIONAL + 1))
      return 1
    fi
  else
    echo -e "   ${CYAN}${INFO} Docker is NOT installed (optional, recommended for container tests)${NC}"
    MISSING_OPTIONAL=$((MISSING_OPTIONAL + 1))
    return 2
  fi
}

# Run all checks
run_all_checks() {
  MISSING_ESSENTIAL=0
  MISSING_OPTIONAL=0
  
  check_git
  echo ""
  check_github
  echo ""
  check_node
  echo ""
  check_npm
  echo ""
  check_firebase
  echo ""
  check_gcloud
  echo ""
  check_docker
  echo ""

  echo -e "${BOLD}-----------------------------------------------------------------${NC}"
  if [ $MISSING_ESSENTIAL -eq 0 ]; then
    echo -e "${BOLD}${GREEN}${CHECK} All essential applications are installed!${NC}"
  else
    echo -e "${BOLD}${RED}${CROSS} Missing $MISSING_ESSENTIAL essential application(s).${NC}"
    echo -e "Run this script with the ${BOLD}--install${NC} or ${BOLD}-i${NC} flag to attempt automated installation."
  fi
  
  if [ $MISSING_OPTIONAL -gt 0 ]; then
    echo -e "${YELLOW}${INFO} Optional application status: $MISSING_OPTIONAL item(s) pending setup.${NC}"
  fi
  echo -e "${BOLD}-----------------------------------------------------------------${NC}"
}

# Automated Installation Function
install_requirements() {
  echo -e "${BOLD}${YELLOW}Starting automated installation...${NC}\n"
  
  if [ "$OS_TYPE" = "Darwin" ]; then
    # macOS Setup
    if ! command -v brew >/dev/null 2>&1; then
      echo -e "${CYAN}Homebrew not detected. Installing Homebrew...${NC}"
      /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
      # Load homebrew for current session
      eval "$(/opt/homebrew/bin/brew shellenv 2>/dev/null || /usr/local/bin/brew shellenv 2>/dev/null)"
    fi
    
    echo -e "${CYAN}Installing core development tools via Homebrew...${NC}"
    brew update
    
    if ! command -v git >/dev/null 2>&1; then
      echo -e "${MAGENTA}Installing Git...${NC}"
      brew install git
    fi
    
    if ! command -v node >/dev/null 2>&1; then
      echo -e "${MAGENTA}Installing Node.js 20...${NC}"
      brew install node@20
      brew link --overwrite node@20
    fi
    
    if ! command -v gh >/dev/null 2>&1; then
      echo -e "${MAGENTA}Installing GitHub CLI...${NC}"
      brew install gh
    fi
    
    if ! command -v gcloud >/dev/null 2>&1; then
      echo -e "${MAGENTA}Installing Google Cloud SDK...${NC}"
      brew install --cask google-cloud-sdk
    fi

  elif [ "$OS_TYPE" = "Linux" ]; then
    # Linux (Ubuntu/Debian) Setup
    if command -v apt-get >/dev/null 2>&1; then
      echo -e "${CYAN}Updating package index...${NC}"
      sudo apt-get update
      sudo apt-get install -y curl gnupg ca-certificates git

      if ! command -v node >/dev/null 2>&1; then
        echo -e "${MAGENTA}Installing Node.js 20 via NodeSource...${NC}"
        curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
        sudo apt-get install -y nodejs
      fi

      if ! command -v gh >/dev/null 2>&1; then
        echo -e "${MAGENTA}Installing GitHub CLI...${NC}"
        sudo mkdir -p -m 755 /etc/apt/keyrings
        curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/etc/apt/keyrings/githubcli-archive-keyring.gpg 2>/dev/null
        sudo chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg
        echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null
        sudo apt-get update
        sudo apt-get install -y gh
      fi

      if ! command -v gcloud >/dev/null 2>&1; then
        echo -e "${MAGENTA}Installing Google Cloud CLI...${NC}"
        curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | sudo gpg --dearmor -o /usr/share/keyrings/cloud.google.gpg 2>/dev/null
        echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main" | sudo tee -a /etc/apt/sources.list.d/google-cloud-sdk.list
        sudo apt-get update && sudo apt-get install -y google-cloud-cli
      fi
    else
      echo -e "${RED}${CROSS} Automated installer only supports APT (Debian/Ubuntu) package managers currently.${NC}"
      echo -e "Please install the missing tools manually using your system package manager."
      exit 1
    fi
  else
    echo -e "${RED}${CROSS} Automated installer does not support OS: $OS_TYPE${NC}"
    echo -e "Please configure requirements manually."
    exit 1
  fi

  # Install npm globals (independent of OS, once Node/NPM are set up)
  if command -v npm >/dev/null 2>&1; then
    if ! command -v firebase >/dev/null 2>&1; then
      echo -e "${MAGENTA}Installing Firebase CLI globally...${NC}"
      sudo npm install -g firebase-tools || npm install -g firebase-tools
    fi
  else
    echo -e "${YELLOW}${WARN} NPM not ready yet, skipping global Firebase CLI installation.${NC}"
  fi

  echo -e "\n${GREEN}${CHECK} Installation steps finished! Running check to verify...${NC}\n"
  run_all_checks
}

# Parse input flags
case "$1" in
  -i|--install)
    print_header
    install_requirements
    ;;
  -h|--help)
    echo "AAE App Requirements Tool"
    echo "Usage:"
    echo "  ./project_requirements.sh            Check required tools status"
    echo "  ./project_requirements.sh -i         Attempt automated installation of missing requirements"
    echo "  ./project_requirements.sh -h         Display this help menu"
    ;;
  *)
    print_header
    run_all_checks
    ;;
esac
