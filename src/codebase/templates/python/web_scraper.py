# @LITE_DESC: Web scraper with requests + BeautifulSoup, rate limiting, retries, CSS selectors, JSON export
# @LITE_SCENE: A robust web scraper using requests and BeautifulSoup with rate limiting, retry logic, CSS selectors, and JSON export functionality
# @LITE_TAGS: python, scraper, beautifulsoup, requests, web

import requests
from bs4 import BeautifulSoup
from typing import Dict, List, Optional, Any
import time
import json
from pathlib import Path
from datetime import datetime
import logging
from urllib.parse import urljoin, urlparse
import random
from dataclasses import dataclass, asdict
from enum import Enum
import backoff

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

class ScraperError(Enum):
    """Enum for scraper error types"""
    CONNECTION_ERROR = "connection_error"
    TIMEOUT_ERROR = "timeout_error"
    HTTP_ERROR = "http_error"
    PARSE_ERROR = "parse_error"
    RATE_LIMIT_ERROR = "rate_limit_error"

@dataclass
class ScrapedItem:
    """Data class for scraped items"""
    url: str
    title: str
    description: Optional[str] = None
    content: Optional[str] = None
    author: Optional[str] = None
    date: Optional[str] = None
    tags: List[str] = None
    metadata: Dict[str, Any] = None

    def __post_init__(self):
        if self.tags is None:
            self.tags = []
        if self.metadata is None:
            self.metadata = {}

class RateLimiter:
    """Rate limiter to control request frequency"""

    def __init__(self, min_delay: float = 1.0, max_delay: float = 3.0):
        self.min_delay = min_delay
        self.max_delay = max_delay
        self.last_request_time = 0

    def wait(self):
        """Wait for appropriate delay before next request"""
        current_time = time.time()
        time_since_last_request = current_time - self.last_request_time

        delay = random.uniform(self.min_delay, self.max_delay)
        if time_since_last_request < delay:
            sleep_time = delay - time_since_last_request
            logger.debug(f"Rate limiting: sleeping for {sleep_time:.2f}s")
            time.sleep(sleep_time)

        self.last_request_time = time.time()

class RetryStrategy:
    """Retry strategy with exponential backoff"""

    def __init__(self, max_retries: int = 3, base_delay: float = 1.0):
        self.max_retries = max_retries
        self.base_delay = base_delay

    @backoff.on_exception(
        backoff.expo,
        (requests.exceptions.RequestException, requests.exceptions.HTTPError),
        max_tries=3,
        base=1
    )
    def request_with_retry(self, func, *args, **kwargs):
        """Execute function with retry logic"""
        return func(*args, **kwargs)

class WebScraper:
    """A comprehensive web scraper with rate limiting and retries"""

    def __init__(
        self,
        base_url: str,
        rate_limiter: Optional[RateLimiter] = None,
        retry_strategy: Optional[RetryStrategy] = None,
        timeout: int = 10,
        user_agent: str = None
    ):
        self.base_url = base_url
        self.rate_limiter = rate_limiter or RateLimiter()
        self.retry_strategy = retry_strategy or RetryStrategy()
        self.timeout = timeout
        self.session = requests.Session()

        # Set default headers
        headers = {
            'User-Agent': user_agent or 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36',
            'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
            'Accept-Language': 'en-US,en;q=0.5',
            'Connection': 'keep-alive',
        }
        self.session.headers.update(headers)

        self.scraped_items: List[ScrapedItem] = []
        self.failed_urls: List[str] = []

    def _make_request(self, url: str, method: str = 'GET', **kwargs) -> requests.Response:
        """Make HTTP request with rate limiting and retry logic"""
        self.rate_limiter.wait()

        full_url = urljoin(self.base_url, url)
        logger.info(f"Requesting: {full_url}")

        try:
            response = self.retry_strategy.request_with_retry(
                self.session.request,
                method,
                full_url,
                timeout=self.timeout,
                **kwargs
            )
            response.raise_for_status()
            return response
        except requests.exceptions.Timeout:
            logger.error(f"Timeout error for {full_url}")
            raise ScraperError.TIMEOUT_ERROR
        except requests.exceptions.ConnectionError:
            logger.error(f"Connection error for {full_url}")
            raise ScraperError.CONNECTION_ERROR
        except requests.exceptions.HTTPError as e:
            logger.error(f"HTTP error for {full_url}: {e}")
            if e.response.status_code == 429:
                raise ScraperError.RATE_LIMIT_ERROR
            raise ScraperError.HTTP_ERROR
        except Exception as e:
            logger.error(f"Unexpected error for {full_url}: {e}")
            raise ScraperError.PARSE_ERROR

    def parse_html(self, html: str) -> BeautifulSoup:
        """Parse HTML content with BeautifulSoup"""
        return BeautifulSoup(html, 'html.parser')

    def scrape_page(
        self,
        url: str,
        title_selector: str = 'h1',
        content_selector: str = 'div.content',
        metadata_selectors: Dict[str, str] = None
    ) -> Optional[ScrapedItem]:
        """Scrape a single page"""
        try:
            response = self._make_request(url)
            soup = self.parse_html(response.text)

            # Extract title
            title_elem = soup.select_one(title_selector)
            title = title_elem.get_text(strip=True) if title_elem else "No title"

            # Extract content
            content_elem = soup.select_one(content_selector)
            content = content_elem.get_text(strip=True) if content_elem else None

            # Extract metadata
            metadata = {}
            if metadata_selectors:
                for key, selector in metadata_selectors.items():
                    elem = soup.select_one(selector)
                    if elem:
                        metadata[key] = elem.get_text(strip=True)

            # Create scraped item
            item = ScrapedItem(
                url=urljoin(self.base_url, url),
                title=title,
                content=content,
                metadata=metadata
            )

            self.scraped_items.append(item)
            logger.info(f"Successfully scraped: {title}")
            return item

        except Exception as e:
            logger.error(f"Failed to scrape {url}: {e}")
            self.failed_urls.append(url)
            return None

    def scrape_list(
        self,
        url: str,
        item_selector: str,
        link_selector: str = 'a',
        follow_links: bool = True,
        max_items: int = None
    ) -> List[ScrapedItem]:
        """Scrape a list of items from a page"""
        items = []

        try:
            response = self._make_request(url)
            soup = self.parse_html(response.text)

            item_elements = soup.select(item_selector)

            if max_items:
                item_elements = item_elements[:max_items]

            logger.info(f"Found {len(item_elements)} items to scrape")

            for elem in item_elements:
                try:
                    if follow_links:
                        link_elem = elem.select_one(link_selector)
                        if link_elem and link_elem.get('href'):
                            item_url = link_elem['href']
                            item = self.scrape_page(item_url)
                            if item:
                                items.append(item)
                    else:
                        # Extract data directly from list item
                        title = elem.get_text(strip=True)
                        link = elem.select_one(link_selector)
                        href = link.get('href') if link else None

                        item = ScrapedItem(
                            url=urljoin(self.base_url, href) if href else self.base_url,
                            title=title,
                            description=title[:200]
                        )
                        self.scraped_items.append(item)
                        items.append(item)

                except Exception as e:
                    logger.error(f"Failed to process item: {e}")
                    continue

            return items

        except Exception as e:
            logger.error(f"Failed to scrape list from {url}: {e}")
            return []

    def scrape_with_css_selectors(
        self,
        url: str,
        selectors: Dict[str, str],
        multiple: bool = False
    ) -> List[Dict[str, str]]:
        """Scrape data using CSS selectors"""
        try:
            response = self._make_request(url)
            soup = self.parse_html(response.text)

            results = []

            if multiple:
                # Scrape multiple items
                base_elem = soup.select_one(list(selectors.values())[0])
                if base_elem:
                    items = soup.select(list(selectors.values())[0])
                    for item in items:
                        data = {}
                        for key, selector in selectors.items():
                            elem = item.select_one(selector)
                            if elem:
                                data[key] = elem.get_text(strip=True)
                        if data:
                            results.append(data)
            else:
                # Scrape single item
                data = {}
                for key, selector in selectors.items():
                    elem = soup.select_one(selector)
                    if elem:
                        data[key] = elem.get_text(strip=True)
                if data:
                    results.append(data)

            return results

        except Exception as e:
            logger.error(f"Failed to scrape with selectors from {url}: {e}")
            return []

    def export_to_json(
        self,
        filepath: str = None,
        indent: int = 2,
        include_metadata: bool = True
    ) -> str:
        """Export scraped data to JSON file"""
        if filepath is None:
            timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
            filepath = f'scraped_data_{timestamp}.json'

        # Convert scraped items to dictionaries
        data = {
            'scraped_at': datetime.now().isoformat(),
            'base_url': self.base_url,
            'total_items': len(self.scraped_items),
            'failed_urls': self.failed_urls,
            'items': []
        }

        for item in self.scraped_items:
            item_dict = asdict(item)
            if not include_metadata:
                item_dict.pop('metadata', None)
            data['items'].append(item_dict)

        # Write to file
        output_path = Path(filepath)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        with open(output_path, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=indent, ensure_ascii=False)

        logger.info(f"Exported {len(self.scraped_items)} items to {filepath}")
        return str(output_path)

    def get_stats(self) -> Dict[str, Any]:
        """Get scraping statistics"""
        return {
            'total_scraped': len(self.scraped_items),
            'total_failed': len(self.failed_urls),
            'success_rate': len(self.scraped_items) / (len(self.scraped_items) + len(self.failed_urls)) if (len(self.scraped_items) + len(self.failed_urls)) > 0 else 0,
            'failed_urls': self.failed_urls
        }

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.session.close()

# Example usage and demonstration
def main():
    """Example usage of the WebScraper class"""

    # Example 1: Scrape a single page
    print("=== Example 1: Scraping a single page ===")
    scraper = WebScraper(
        base_url="https://example.com",
        rate_limiter=RateLimiter(min_delay=1.0, max_delay=2.0)
    )

    try:
        # Scrape a single page
        item = scraper.scrape_page(
            url="/",
            title_selector="h1",
            content_selector="p",
            metadata_selectors={
                "date": "time",
                "author": ".author"
            }
        )

        if item:
            print(f"Scraped: {item.title}")

        # Get statistics
        stats = scraper.get_stats()
        print(f"Stats: {stats}")

        # Export to JSON
        json_file = scraper.export_to_json("output/scraped_data.json")
        print(f"Data exported to: {json_file}")

    except Exception as e:
        print(f"Error: {e}")

    # Example 2: Scrape a list of articles
    print("\n=== Example 2: Scraping a list of items ===")
    with WebScraper("https://example.com") as list_scraper:
        items = list_scraper.scrape_list(
            url="/articles",
            item_selector="article",
            link_selector="h2 a",
            follow_links=True,
            max_items=5
        )

        print(f"Scraped {len(items)} articles")

        # Export results
        list_scraper.export_to_json("output/articles.json")

    # Example 3: Scrape using CSS selectors
    print("\n=== Example 3: Using CSS selectors ===")
    with WebScraper("https://example.com") as css_scraper:
        data = css_scraper.scrape_with_css_selectors(
            url="/products",
            selectors={
                "title": ".product-title",
                "price": ".product-price",
                "description": ".product-description"
            },
            multiple=True
        )

        print(f"Extracted {len(data)} products")
        for i, product in enumerate(data[:3], 1):
            print(f"Product {i}: {product.get('title', 'N/A')} - {product.get('price', 'N/A')}")

if __name__ == '__main__':
    main()
