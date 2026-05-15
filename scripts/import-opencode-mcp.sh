#!/bin/bash
#==========================================
# Script para importar configuración MCP de OpenCode a Pi
# 
# Usage: ./scripts/import-opencode-mcp.sh [--dry-run] [--output FILE]
#
# Sin argumentos: convierte y escribe a ~/.config/mcp/mcp.json
# --dry-run: solo muestra lo queería sin escribir
# --stdout: muestra el resultado en stdout
# --output FILE: escribe al archivo especificado
#==========================================

set -e

DRY_RUN=false
OUTPUT_FILE=""
USE_STDOUT=false

# Parsear argumentos
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        --stdout)
            USE_STDOUT=true
            shift
            ;;
        *)
            echo "Uso: $0 [--dry-run] [--output FILE] [--stdout]"
            exit 1
            ;;
    esac
done

# Archivos a procesar
OPENCODE_CONFIG="${HOME}/.config/opencode/opencode.json"

# Verificar que existe opencode config
if [[ ! -f "$OPENCODE_CONFIG" ]]; then
    echo "Error: No se encontró configuración de OpenCode en $OPENCODE_CONFIG" >&2
    exit 1
fi

# Verificar jq
if ! command -v jq &> /dev/null; then
    echo "Error: jq es requerido pero no está instalado" >&2
    exit 1
fi

# Crear archivo temporal
TEMP_FILE=$(mktemp)
trap "rm -f $TEMP_FILE" EXIT

# Generar el JSON usando un script jq inline
# OpenCode usa "environment", MCP usa "env"
jq '
def convert_server($v):
  if ($v.type == "remote" or ($v.url != null)) then
    { url: $v.url } +
    (if ($v.headers | type) == "object" then { headers: $v.headers } else {} end)
  else
    { command: ($v.command | if (type == "array") then .[0] else . end) } +
    (if ($v.command | type == "array") and (($v.command | length) > 1) 
     then { args: ($v.command[1:] | if length > 0 then . else null end) } else {} end) +
    (if ($v.environment != null) and (($v.environment | type) == "object") 
     then { env: $v.environment } else {} end)
  end;

{ mcpServers: (.mcp | to_entries | map({key: .key, value: convert_server(.value)}) | from_entries) }
' "$OPENCODE_CONFIG" > "$TEMP_FILE"

# Verificar que el JSON es válido
if ! jq empty "$TEMP_FILE" 2>/dev/null; then
    echo "Error: La conversión produjo JSON inválido" >&2
    exit 1
fi

if [[ "$DRY_RUN" == "true" ]]; then
    echo ""
    echo "=== DRY RUN: No se escribió ningún archivo ==="
    echo ""
    echo "Contenido convertido:"
    cat "$TEMP_FILE"
elif [[ "$USE_STDOUT" == "true" ]]; then
    cat "$TEMP_FILE"
else
    if [[ -z "$OUTPUT_FILE" ]]; then
        OUTPUT_FILE="${HOME}/.config/mcp/mcp.json"
    fi
    
    # Crear directorio si no existe
    mkdir -p "$(dirname "$OUTPUT_FILE")"
    
    # Backup si ya existe
    if [[ -f "$OUTPUT_FILE" ]]; then
        cp "$OUTPUT_FILE" "${OUTPUT_FILE}.bak"
        echo "Backup creado: ${OUTPUT_FILE}.bak"
    fi
    
    cat "$TEMP_FILE" > "$OUTPUT_FILE"
    
    echo "✓ Configuración escrita a: $OUTPUT_FILE"
    echo ""
    echo "Servers importados:"
    jq -r '.mcpServers | keys[]' "$OUTPUT_FILE" | while read -r name; do
        echo "  - $name"
    done
    echo ""
    echo "Próximos pasos:"
    echo "  1. Revisa el archivo: $OUTPUT_FILE"
    echo "  2. Ejecuta 'pi-mcp-adapter init' para importar compatibilidad"
    echo "  3. Reinicia pi"
fi
