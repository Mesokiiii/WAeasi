# `__NAME__`

Generated from the `waeasi` Python SDK handler template.

## Develop

```bash
pip install -e .
waeasi build --dev          # skip wizer for fast iteration
```

## Build for production

```bash
export WAEASI_SIGN_KEY=$(cat my-key.hex)
waeasi build                # emits dist/__NAME__.waeasi-bundle
waeasi deploy
```

## Test

```bash
pip install -e ".[dev]"
pytest
```

`@define_handler()` registers the coroutine globally; in tests you can
import `__NAME__.handler.handle` and call it directly with a fixture
``Request`` and ``ExecutionContext``.
