import asyncio
import aiohttp


async def fetch(session, bv, params=None, headers=None, timeout=5):
    try:
        url = f"https://api.bilibili.com/x/web-interface/view?bvid={bv}"
        async with session.get(
                url,
                params=params,
                headers=headers,
                timeout=aiohttp.ClientTimeout(total=timeout),
                # proxy="http://127.0.0.1:10809"
        ) as response:
            response.raise_for_status()
            data = await response.json()
            return {
                "bv": bv,
                "mid": data["data"]["owner"]["mid"]
            }
    except Exception as e:
        return {
            "bv": bv,
            "mid": None
        }


async def query_all(bv_list: list):
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0"
    }
    async with aiohttp.ClientSession() as session:
        tasks = [fetch(session, bv, headers=headers) for bv in bv_list]
        results = await asyncio.gather(*tasks)
        return results


def async_query(bv_list: list) -> (dict, list):
    return asyncio.run(query_all(bv_list))
