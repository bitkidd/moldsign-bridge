import { IconContext } from '@phosphor-icons/react';

import { HomeScreen } from '#screens/home_screen';
const iconDefaults = { weight: 'regular', 'aria-hidden': true } as const;
export function App() {
  return (
    <IconContext.Provider value={iconDefaults}>
      <HomeScreen />
    </IconContext.Provider>
  );
}
