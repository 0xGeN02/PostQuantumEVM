FROM ghcr.io/astral-sh/uv:python3.12-bookworm-slim

WORKDIR /qiskit-api

# Configurar variables de entorno para uv
ENV UV_COMPILE_BYTECODE=1
ENV UV_LINK_MODE=copy
ENV UV_NO_DEV=1
ENV UV_TOOL_BIN_DIR=/usr/local/bin

COPY pyproject.toml uv.lock ./

RUN uv lock && uv sync --no-dev --frozen

ENV PATH="/qiskit-api/.venv/bin:$PATH"

EXPOSE 8888

CMD ["/qiskit-api/.venv/bin/uvicorn", "src.main:app", "--host", "0.0.0.0", "--port", "8888"]