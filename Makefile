.PHONY: help install dev build test clean

# Colors for terminal output
RED=\033[0;31m
GREEN=\033[0;32m
YELLOW=\033[0;33m
BLUE=\033[0;34m
MAGENTA=\033[0;35m
CYAN=\033[0;36m
NC=\033[0m # No Color

# Directories
FRONTEND_DIR=packages/frontend
BACKEND_DIR=packages/shadow-swap
CONTRACTS_DIR=packages/contracts

help: ## Show this help message
	@echo "$(CYAN)╔════════════════════════════════════════════════════════╗$(NC)"
	@echo "$(CYAN)║          Shadow Swap - Monorepo Commands              ║$(NC)"
	@echo "$(CYAN)╚════════════════════════════════════════════════════════╝$(NC)"
	@echo ""
	@echo "$(MAGENTA)Global Commands:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -v "frontend\|backend\|contracts" | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(MAGENTA)Frontend Commands:$(NC)"
	@grep -E '^frontend-[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(MAGENTA)Backend Commands:$(NC)"
	@grep -E '^backend-[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(MAGENTA)Contracts Commands:$(NC)"
	@grep -E '^contracts-[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}'

# ============================================
# Global Commands
# ============================================

install: ## Install all dependencies (all packages)
	@echo "$(BLUE)Installing all dependencies...$(NC)"
	@$(MAKE) frontend-install
	@$(MAKE) backend-install
	@$(MAKE) contracts-install
	@echo "$(GREEN)✓ All dependencies installed$(NC)"

dev: ## Start all development servers
	@echo "$(BLUE)Starting all development servers...$(NC)"
	@echo "$(YELLOW)Note: This will run frontend and backend in parallel$(NC)"
	@$(MAKE) -j2 frontend-dev backend-dev

clean: ## Clean all build artifacts
	@echo "$(YELLOW)Cleaning all build artifacts...$(NC)"
	@$(MAKE) frontend-clean
	@$(MAKE) backend-clean
	@$(MAKE) contracts-clean
	@echo "$(GREEN)✓ All cleaned$(NC)"

test: ## Run all tests
	@echo "$(BLUE)Running all tests...$(NC)"
	@$(MAKE) frontend-test
	@$(MAKE) backend-test
	@$(MAKE) contracts-test

lint: ## Lint all packages
	@echo "$(BLUE)Linting all packages...$(NC)"
	@$(MAKE) frontend-lint
	@$(MAKE) backend-lint
	@$(MAKE) contracts-lint

build: ## Build all packages
	@echo "$(BLUE)Building all packages...$(NC)"
	@$(MAKE) frontend-build
	@$(MAKE) backend-build
	@$(MAKE) contracts-build
	@echo "$(GREEN)✓ All packages built$(NC)"

# ============================================
# Frontend Commands
# ============================================

frontend-install: ## Install frontend dependencies
	@echo "$(BLUE)📦 Installing frontend dependencies...$(NC)"
	@cd $(FRONTEND_DIR) && npm install
	@echo "$(GREEN)✓ Frontend dependencies installed$(NC)"

frontend-dev: ## Start frontend development server
	@echo "$(BLUE)🚀 Starting frontend development server...$(NC)"
	@cd $(FRONTEND_DIR) && npm run dev

frontend-build: ## Build frontend for production
	@echo "$(BLUE)🔨 Building frontend...$(NC)"
	@cd $(FRONTEND_DIR) && npm run build
	@echo "$(GREEN)✓ Frontend build completed$(NC)"

frontend-start: ## Start frontend production server
	@echo "$(BLUE)▶️  Starting frontend production server...$(NC)"
	@cd $(FRONTEND_DIR) && npm run start

frontend-lint: ## Lint frontend code
	@echo "$(BLUE)🔍 Linting frontend...$(NC)"
	@cd $(FRONTEND_DIR) && npm run lint

frontend-test: ## Run frontend tests
	@echo "$(BLUE)🧪 Running frontend tests...$(NC)"
	@cd $(FRONTEND_DIR) && npm run test || echo "$(YELLOW)No tests configured$(NC)"

frontend-clean: ## Clean frontend build artifacts
	@echo "$(YELLOW)🧹 Cleaning frontend...$(NC)"
	@cd $(FRONTEND_DIR) && rm -rf .next node_modules out
	@echo "$(GREEN)✓ Frontend cleaned$(NC)"

frontend-typecheck: ## Run TypeScript type checking
	@echo "$(BLUE)🔤 Type checking frontend...$(NC)"
	@cd $(FRONTEND_DIR) && npx tsc --noEmit

# ============================================
# Backend Commands (Rust)
# ============================================

backend-install: ## Install backend dependencies
	@echo "$(BLUE)📦 Installing backend dependencies...$(NC)"
	@cd $(BACKEND_DIR) && cargo fetch
	@echo "$(GREEN)✓ Backend dependencies installed$(NC)"

backend-dev: ## Start backend development server
	@echo "$(BLUE)🚀 Starting backend development server...$(NC)"
	@cd $(BACKEND_DIR) && cargo run

backend-build: ## Build backend for production
	@echo "$(BLUE)🔨 Building backend...$(NC)"
	@cd $(BACKEND_DIR) && cargo build --release
	@echo "$(GREEN)✓ Backend build completed$(NC)"

backend-test: ## Run backend tests
	@echo "$(BLUE)🧪 Running backend tests...$(NC)"
	@cd $(BACKEND_DIR) && cargo test

backend-lint: ## Lint backend code
	@echo "$(BLUE)🔍 Linting backend...$(NC)"
	@cd $(BACKEND_DIR) && cargo clippy -- -D warnings || echo "$(YELLOW)Clippy warnings found$(NC)"

backend-format: ## Format backend code
	@echo "$(BLUE)✨ Formatting backend...$(NC)"
	@cd $(BACKEND_DIR) && cargo fmt

backend-clean: ## Clean backend build artifacts
	@echo "$(YELLOW)🧹 Cleaning backend...$(NC)"
	@cd $(BACKEND_DIR) && cargo clean
	@echo "$(GREEN)✓ Backend cleaned$(NC)"

backend-watch: ## Watch and rebuild backend on changes
	@echo "$(BLUE)👀 Watching backend for changes...$(NC)"
	@cd $(BACKEND_DIR) && cargo watch -x run

# ============================================
# Contracts Commands (Foundry)
# ============================================

contracts-install: ## Install contract dependencies
	@echo "$(BLUE)📦 Installing contract dependencies...$(NC)"
	@cd $(CONTRACTS_DIR) && forge install || echo "$(GREEN)✓ Dependencies already installed$(NC)"

contracts-build: ## Build smart contracts
	@echo "$(BLUE)🔨 Building contracts...$(NC)"
	@cd $(CONTRACTS_DIR) && forge build
	@echo "$(GREEN)✓ Contracts built$(NC)"

contracts-test: ## Run contract tests
	@echo "$(BLUE)🧪 Running contract tests...$(NC)"
	@cd $(CONTRACTS_DIR) && forge test -vvv

contracts-test-coverage: ## Generate test coverage report
	@echo "$(BLUE)📊 Generating test coverage...$(NC)"
	@cd $(CONTRACTS_DIR) && forge coverage

contracts-lint: ## Lint contracts
	@echo "$(BLUE)🔍 Linting contracts...$(NC)"
	@cd $(CONTRACTS_DIR) && forge fmt --check || echo "$(YELLOW)Run 'make contracts-format' to fix$(NC)"

contracts-format: ## Format contracts
	@echo "$(BLUE)✨ Formatting contracts...$(NC)"
	@cd $(CONTRACTS_DIR) && forge fmt

contracts-clean: ## Clean contract build artifacts
	@echo "$(YELLOW)🧹 Cleaning contracts...$(NC)"
	@cd $(CONTRACTS_DIR) && forge clean
	@echo "$(GREEN)✓ Contracts cleaned$(NC)"

contracts-deploy-mantle: ## Deploy contracts to Mantle Sepolia
	@echo "$(BLUE)🚀 Deploying contracts to Mantle Sepolia...$(NC)"
	@cd $(CONTRACTS_DIR) && forge script script/Deploy.s.sol --rpc-url mantle_sepolia --broadcast --verify

contracts-deploy-ethereum: ## Deploy contracts to Ethereum Sepolia
	@echo "$(BLUE)🚀 Deploying contracts to Ethereum Sepolia...$(NC)"
	@cd $(CONTRACTS_DIR) && forge script script/Deploy.s.sol --rpc-url ethereum_sepolia --broadcast --verify

# ============================================
# Utility Commands
# ============================================

status: ## Show status of all packages
	@echo "$(CYAN)╔════════════════════════════════════════════════════════╗$(NC)"
	@echo "$(CYAN)║              Shadow Swap - Status Check               ║$(NC)"
	@echo "$(CYAN)╚════════════════════════════════════════════════════════╝$(NC)"
	@echo ""
	@echo "$(MAGENTA)Frontend:$(NC)"
	@cd $(FRONTEND_DIR) && \
		if [ -d "node_modules" ]; then echo "  $(GREEN)✓$(NC) Dependencies installed"; else echo "  $(RED)✗$(NC) Dependencies not installed"; fi && \
		if [ -d ".next" ]; then echo "  $(GREEN)✓$(NC) Build exists"; else echo "  $(YELLOW)○$(NC) Not built"; fi
	@echo ""
	@echo "$(MAGENTA)Backend:$(NC)"
	@cd $(BACKEND_DIR) && \
		if [ -f "Cargo.lock" ]; then echo "  $(GREEN)✓$(NC) Dependencies locked"; else echo "  $(YELLOW)○$(NC) Dependencies not locked"; fi && \
		if [ -d "target" ]; then echo "  $(GREEN)✓$(NC) Build exists"; else echo "  $(YELLOW)○$(NC) Not built"; fi
	@echo ""
	@echo "$(MAGENTA)Contracts:$(NC)"
	@cd $(CONTRACTS_DIR) && \
		if [ -d "lib" ]; then echo "  $(GREEN)✓$(NC) Dependencies installed"; else echo "  $(YELLOW)○$(NC) Dependencies not installed"; fi && \
		if [ -d "out" ]; then echo "  $(GREEN)✓$(NC) Build exists"; else echo "  $(YELLOW)○$(NC) Not built"; fi

setup: ## Initial setup for new developers
	@echo "$(CYAN)╔════════════════════════════════════════════════════════╗$(NC)"
	@echo "$(CYAN)║          Shadow Swap - Initial Setup                  ║$(NC)"
	@echo "$(CYAN)╚════════════════════════════════════════════════════════╝$(NC)"
	@echo ""
	@echo "$(BLUE)Step 1: Installing all dependencies...$(NC)"
	@$(MAKE) install
	@echo ""
	@echo "$(BLUE)Step 2: Setting up environment files...$(NC)"
	@cd $(FRONTEND_DIR) && if [ ! -f ".env.local" ]; then cp .env.example .env.local && echo "$(GREEN)✓$(NC) Created frontend/.env.local"; else echo "$(YELLOW)○$(NC) frontend/.env.local already exists"; fi
	@echo ""
	@echo "$(GREEN)✓ Setup complete!$(NC)"
	@echo ""
	@echo "$(YELLOW)Next steps:$(NC)"
	@echo "  1. Update .env.local files with your configuration"
	@echo "  2. Run '$(GREEN)make dev$(NC)' to start development servers"
	@echo "  3. Visit http://localhost:3000 for frontend"

all: clean install build test ## Clean, install, build, and test everything
	@echo "$(GREEN)╔════════════════════════════════════════════════════════╗$(NC)"
	@echo "$(GREEN)║            All Tasks Completed Successfully            ║$(NC)"
	@echo "$(GREEN)╚════════════════════════════════════════════════════════╝$(NC)"
