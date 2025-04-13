import asyncio
from src.utils import demo_quote

def main():
    """Entry point that properly handles async execution"""
    try:
        asyncio.get_event_loop().run_until_complete(demo_quote())
    except Exception as e:
        print(f"Error in main: {e}")

if __name__ == "__main__":
    main()
