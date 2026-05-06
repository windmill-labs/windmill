#!/bin/bash
#
# Windmill Nix container build system
#

set -euo pipefail

DEFAULT_NIX_FILE="container/default.nix"

if [[ -t 1 ]]; then
    RED='\033[0;31m' GREEN='\033[0;32m' YELLOW='\033[1;33m' BLUE='\033[0;34m' PURPLE='\033[0;35m' CYAN='\033[0;36m' NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' PURPLE='' CYAN='' NC=''
fi

log_info() { [[ "${QUIET:-0}" != "1" ]] && echo -e "${BLUE}ℹ️  $1${NC}" >&2; }
log_success() { echo -e "${GREEN}✅ $1${NC}" >&2; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}" >&2; }
log_error() { echo -e "${RED}❌ $1${NC}" >&2; }
log_header() { echo -e "${PURPLE}🏗️  $1${NC}" >&2; }
log_step() { [[ "${QUIET:-0}" != "1" ]] && echo -e "${CYAN}📦 $1${NC}" >&2; }

parse_args() {
    VERSION="${VERSION:-}"
    TAG="${TAG:-}"
    IMAGE_NAME="${IMAGE_NAME:-windmill}"
    PROJECT_ROOT="${PROJECT_ROOT:-$(git rev-parse --show-toplevel)}"
    BUILDER_TAG="${BUILDER_TAG:-}"
    NIX_FILE="${NIX_FILE:-$DEFAULT_NIX_FILE}"
    BUILDER_FILE="${BUILDER_FILE:-build/Dockerfile.nix-builder}"
    OUTPUT_FILE="${OUTPUT_FILE:-}"
    CORES="${CORES:-}"
    NO_CACHE="false"
    NO_CLEAN="false"
    DRY_RUN="false"
    QUIET="${QUIET:-0}"
    WITH_GIT="${WITH_GIT:-true}"
    WITH_KUBECTL="${WITH_KUBECTL:-true}"
    WITH_HELM="${WITH_HELM:-true}"
    WITH_POWERSHELL="${WITH_POWERSHELL:-false}"

    while [[ $# -gt 0 ]]; do
        case $1 in
            -v|--version)
                VERSION="$2"
                shift 2
                ;;
            -t|--tag)
                TAG="$2"
                shift 2
                ;;
            -n|--name)
                IMAGE_NAME="$2"
                shift 2
                ;;
            -b|--builder)
                BUILDER_TAG="$2"
                shift 2
                ;;
            -f|--file)
                NIX_FILE="$2"
                shift 2
                ;;
            --builder-file)
                BUILDER_FILE="$2"
                shift 2
                ;;
            -o|--output)
                OUTPUT_FILE="$2"
                shift 2
                ;;
            -c|--cores)
                CORES="$2"
                shift 2
                ;;
            --no-cache)
                NO_CACHE="true"
                shift
                ;;
            --no-clean)
                NO_CLEAN="true"
                shift
                ;;
            --dry-run)
                DRY_RUN="true"
                shift
                ;;
            -q|--quiet)
                QUIET="1"
                shift
                ;;
            --with-git)
                WITH_GIT="true"
                shift
                ;;
            --without-git)
                WITH_GIT="false"
                shift
                ;;
            --with-kubectl)
                WITH_KUBECTL="true"
                shift
                ;;
            --without-kubectl)
                WITH_KUBECTL="false"
                shift
                ;;
            --with-helm)
                WITH_HELM="true"
                shift
                ;;
            --without-helm)
                WITH_HELM="false"
                shift
                ;;
            --with-powershell)
                WITH_POWERSHELL="true"
                shift
                ;;
            --without-powershell)
                WITH_POWERSHELL="false"
                shift
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

show_help() {
    cat << 'EOF'
Windmill Nix container build system

USAGE:
    ./build-container-nix.sh [OPTIONS]

REQUIRED OPTIONS:
    -v, --version VERSION     Build version tag (default: from version.txt)

OPTIONAL OPTIONS:
    -n, --name IMAGE_NAME      Container image name (default: windmill)
    -t, --tag TAG             Docker image tag (default: VERSION)
    -b, --builder TAG         Custom builder container tag (default: IMAGE_NAME-builder:VERSION)
    -f, --file FILE           Nix file to use (default: container/default.nix)
    -o, --output FILE         Output tarball path (default: IMAGE_NAME-VERSION.tar.gz)
    -c, --cores NUM           Number of CPU cores for build (default: auto-detect)
    --no-cache                Force rebuild without cache
    --no-clean               Skip builder container cleanup
    --dry-run                Show what would be executed without running
    -q, --quiet              Minimal output
    --with-git / --without-git           Include git (default: with)
    --with-kubectl / --without-kubectl   Include kubectl (default: with)
    --with-helm / --without-helm         Include helm (default: with)
    --with-powershell / --without-powershell  Include powershell (default: without)
    -h, --help               Show this help message

EXAMPLES:
    ./build-container-nix.sh -v 1.441.0
    ./build-container-nix.sh -v 1.441.0 --without-kubectl --without-helm
    VERSION=1.441.0 ./build-container-nix.sh

ENVIRONMENT VARIABLES:
    VERSION       Build version
    IMAGE_NAME    Container name
    TAG           Docker image tag
    PROJECT_ROOT  Root directory of windmill project
EOF
}

resolve_parameters() {
    log_step "Resolving build parameters..."
    
    if [[ -z "$VERSION" ]]; then
        VERSION=$(cat "$PROJECT_ROOT/version.txt" | tr -d '\n')
        log_info "Version from version.txt: $VERSION"
    fi
    
    if [[ -z "$TAG" ]]; then
        TAG="$VERSION"
    fi
    
    if [[ -z "$BUILDER_TAG" ]]; then
        BUILDER_TAG="$IMAGE_NAME-builder:$VERSION"
    fi
    
    if [[ -z "$OUTPUT_FILE" ]]; then
        OUTPUT_FILE="$IMAGE_NAME-$VERSION.tar.gz"
    fi
    
    if [[ -z "$CORES" ]]; then
        if command -v nproc >/dev/null 2>&1; then
            CORES=$(nproc)
        else
            CORES="1"
        fi
    fi
    
    FULL_IMAGE_NAME="$IMAGE_NAME:$TAG"
    
    log_info "Version: $VERSION"
    log_info "Image: $FULL_IMAGE_NAME"
    log_info "Cores: $CORES"
    log_info "With git: $WITH_GIT"
    log_info "With kubectl: $WITH_KUBECTL"
    log_info "With helm: $WITH_HELM"
    log_info "With powershell: $WITH_POWERSHELL"
}

validate_inputs() {
    log_step "Validating inputs..."
    
    if [[ ! -f "$NIX_FILE" ]]; then
        log_error "Nix file not found: $NIX_FILE"
        exit 1
    fi
    
    if ! command -v docker >/dev/null 2>&1; then
        log_error "Docker is required but not found"
        exit 1
    fi
    
    log_success "Input validation passed"
}

validate_builder_dockerfile() {
    if [[ ! -f "$BUILDER_FILE" ]]; then
        log_error "Builder Dockerfile not found: $BUILDER_FILE"
        exit 1
    fi
    
    echo "$BUILDER_FILE"
}

execute_build() {
    log_header "Starting Nix container build process"
    
    local builder_dockerfile
    builder_dockerfile=$(validate_builder_dockerfile)
    
    log_step "Building builder container using: $builder_dockerfile"
    
    if [[ "$DRY_RUN" = "true" ]]; then
        log_info "[DRY-RUN] Would build builder container with:"
        log_info "  Dockerfile: $builder_dockerfile"
        log_info "  Tag: $BUILDER_TAG"
        log_info "  Nix file: $NIX_FILE"
        log_info "  Version: $VERSION"
        return 0
    fi
    
    local build_args=(
        --build-arg "VERSION=$VERSION"
        --build-arg "BUILD_CORES=$CORES"
        --build-arg "NIX_FILE=$NIX_FILE"
    )
    
    if [[ "$NO_CACHE" = "true" ]]; then
        build_args+=(--no-cache)
    fi
    
    local empty_context="/tmp/nix-builder-context-$$"
    mkdir -p "$empty_context"
    
    docker build "${build_args[@]}" \
        -f "$builder_dockerfile" \
        -t "$BUILDER_TAG" \
        "$empty_context" || {
        rm -rf "$empty_context"
        log_error "Builder container build failed"
        exit 1
    }
    
    rm -rf "$empty_context"
    
    log_success "Builder container built successfully"
    
    log_step "Running Nix build with mounted source..."
    
    docker run --rm \
        -v "$PROJECT_ROOT:/project-root" \
        -v "$PROJECT_ROOT:/source" \
        -v "$(pwd):/output" \
        -e "VERSION=$VERSION" \
        -e "IMAGE_NAME=$IMAGE_NAME" \
        -e "PROJECT_ROOT=/project-root" \
        -e "BUILD_CORES=$CORES" \
        -e "NIX_FILE=/source/$NIX_FILE" \
        -e "WITH_GIT=$WITH_GIT" \
        -e "WITH_KUBECTL=$WITH_KUBECTL" \
        -e "WITH_HELM=$WITH_HELM" \
        -e "WITH_POWERSHELL=$WITH_POWERSHELL" \
        -e "NO_CACHE=$NO_CACHE" \
        "$BUILDER_TAG" \
        sh -c "
            nix-build \$NIX_FILE -A containerImage \
                --option sandbox false \
                --cores \$BUILD_CORES \
                --argstr version \"\$VERSION\" \
                --argstr imageName \"\$IMAGE_NAME\" \
                --argstr projectRoot \"\$PROJECT_ROOT\" \
                --arg withGit \$WITH_GIT \
                --arg withKubectl \$WITH_KUBECTL \
                --arg withHelm \$WITH_HELM \
                --arg withPowershell \$WITH_POWERSHELL && \
            cp result /output/$OUTPUT_FILE
        " || {
        log_error "Build failed"
        exit 1
    }
    
    log_step "Loading Docker image..."
    docker load < "$OUTPUT_FILE" || {
        log_error "Failed to load Docker image from: $OUTPUT_FILE"
        exit 1
    }
    
    log_success "Build process completed successfully"
}

show_summary() {
    echo ""
    log_header "🎉 BUILD SUMMARY"
    echo "==================="
    echo "✅ Version: $VERSION"
    echo "✅ Image: $FULL_IMAGE_NAME"
    echo "✅ Output: $OUTPUT_FILE"
    echo "✅ Size: $(du -h "$OUTPUT_FILE" 2>/dev/null | cut -f1 || echo "unknown")"
    echo ""
    echo "🚀 Next Steps:"
    echo "• Test: docker run --rm -p 8000:8000 $FULL_IMAGE_NAME"
    echo "• Inspect: docker history $FULL_IMAGE_NAME"
    echo ""
}

main() {
    parse_args "$@"
    resolve_parameters
    validate_inputs
    execute_build
    show_summary
    
    if [[ "$DRY_RUN" != "true" ]]; then
        log_success "Nix container build completed successfully! 🎯"
    fi
}

if [[ "${BASH_SOURCE[0]}" = "${0}" ]]; then
    main "$@"
fi
