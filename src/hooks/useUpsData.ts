import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useUpsStore } from '../store/upsStore';
import { UpsData } from '../types/ups';

export function useUpsData() {
  const setUpsData = useUpsStore((state) => state.setUpsData);
  const setError = useUpsStore((state) => state.setError);

  useEffect(() => {
    const unlisten = listen<UpsData>('ups-update', (event) => {
      console.log('Received UPS update:', event.payload);
      setUpsData(event.payload);
      setError(null);
    });
    const unlistenError = listen<string>('ups-error', (event) => {
      console.warn('UPS poll error:', event.payload);
      setError(event.payload);
    });

    return () => {
      unlisten.then((f) => f());
      unlistenError.then((f) => f());
    };
  }, [setUpsData, setError]);
}
