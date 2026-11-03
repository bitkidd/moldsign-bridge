import { AppMark } from '#components/layout/app_mark';
export function AppHeader() {
  return (
    <header className='app-header'>
      <AppMark />
      <div>
        <h1>MoldSign Bridge</h1>
        <p>Your key to government services</p>
      </div>
      <span className='version'>PREVIEW</span>
    </header>
  );
}
