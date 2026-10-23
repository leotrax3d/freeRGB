import { Client, Account, Functions } from 'appwrite';

export const projectName = 'freeRGB';
export const client = new Client()
  .setEndpoint('https://fra.cloud.appwrite.io/v1')
  .setProject('6adb5259001299138e7b');

export const account = new Account(client);
export const functions = new Functions(client);

// ES modules initialize once per page load. No session or secret is needed.
export const startupPing = client.ping().then(
  () => { console.info('Appwrite connected: freeRGB'); return true; },
  () => { console.warn('Appwrite connection check failed'); return false; },
);
