import { ConnectionDetails } from '#components/connection/connection_details';
import { ConnectionStatus } from '#components/connection/connection_status';
import { AppFooter } from '#components/layout/app_footer';
import { AppHeader } from '#components/layout/app_header';
import { LoginDialog } from '#components/login/login_dialog';
import { TokenList } from '#components/token/token_list';
import { useBridge } from '#hooks/use_bridge';
import { Notice } from '#ui/notice';
export function HomeScreen() {
  const bridge = useBridge();
  const { driver, service, busy } = bridge;
  const ready = Boolean(driver && service?.running && service.trusted);
  return (
    <>
      <AppHeader />
      <main>
        <ConnectionStatus
          driver={driver}
          service={service}
          busy={busy}
          onAction={bridge.primaryAction}
        />
        <TokenList
          tokens={bridge.tokens}
          message={bridge.tokenMessage}
          scanning={bridge.scanning}
          disabled={busy || Boolean(service?.pending)}
          onRefresh={bridge.scan}
        />
        <Notice notice={bridge.notice} />
        <ConnectionDetails driver={driver} service={service} ready={ready} />
      </main>
      <AppFooter ready={ready} />
      <LoginDialog
        key={service?.pending?.id ?? 'idle'}
        pending={service?.pending ?? null}
        busy={busy}
        approve={bridge.approve}
        reject={bridge.reject}
      />
    </>
  );
}
