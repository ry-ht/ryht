import { useRef, useEffect } from 'react';

/**
 * Custom hook to set document title
 * Replacement for react-helmet-async to support React 19
 *
 * @param title - The title to set for the document
 * @param restoreOnUnmount - Whether to restore the previous title when component unmounts
 */
export function useDocumentTitle(title: string, restoreOnUnmount = false): void {
  const prevTitleRef = useRef(document.title);

  useEffect(() => {
    const previousTitle = prevTitleRef.current;

    if (title) {
      document.title = title;
    }

    return () => {
      if (restoreOnUnmount) {
        document.title = previousTitle;
      }
    };
  }, [title, restoreOnUnmount]);
}

/**
 * Custom hook to set document meta tags
 *
 * @param meta - Object with meta tag properties
 */
export function useDocumentMeta(meta: Record<string, string>): void {
  useEffect(() => {
    const metaTags: HTMLMetaElement[] = [];

    Object.entries(meta).forEach(([name, content]) => {
      let metaTag = document.querySelector<HTMLMetaElement>(
        `meta[name="${name}"], meta[property="${name}"]`
      );

      if (!metaTag) {
        metaTag = document.createElement('meta');
        metaTag.setAttribute(
          name.startsWith('og:') || name.startsWith('twitter:') ? 'property' : 'name',
          name
        );
        document.head.appendChild(metaTag);
        metaTags.push(metaTag);
      }

      metaTag.setAttribute('content', content);
    });

    return () => {
      metaTags.forEach((tag) => {
        if (tag.parentNode) {
          tag.parentNode.removeChild(tag);
        }
      });
    };
  }, [meta]);
}
