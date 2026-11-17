import { useEffect, useRef } from 'react';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { useUpsStore } from '../store/upsStore';
import { t } from '../i18n';

export function useNotifications() {
  const events = useUpsStore((state) => state.events);
  const lastEventId = useRef<string | null>(null);

  useEffect(() => {
    const checkPermission = async () => {
      let permissionGranted = await isPermissionGranted();
      if (!permissionGranted) {
        const permission = await requestPermission();
        permissionGranted = permission === 'granted';
      }
    };
    checkPermission();
  }, []);

  useEffect(() => {
    if (events.length > 0) {
      const latestEvent = events[0];

      // Only notify if it's a new event and not the one we just handled
      if (latestEvent.id !== lastEventId.current) {
        lastEventId.current = latestEvent.id;

        // Map common status transitions to human-friendly messages
        // Status words are compared as whole words ("OL CHRG" → "OB DISCHRG" is still a power loss).
        const [from = '', to = ''] = latestEvent.message.split(' → ');
        const has = (s: string, flag: string) => s.split(' ').includes(flag);
        let detail = latestEvent.message;
        if (has(from, 'OL') && has(to, 'OB')) detail = t('notify.powerLost');
        if (has(from, 'OB') && has(to, 'OL')) detail = t('notify.powerRestored');
        if (has(to, 'LB') && !has(from, 'LB')) detail = t('notify.lowBattery');

        sendNotification({
          title: latestEvent.type === 'warning' ? t('notify.alertTitle') : t('notify.statusTitle'),
          body: detail,
        });
      }
    }
  }, [events]);
}
