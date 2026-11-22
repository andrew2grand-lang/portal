; SeamDrop NSIS installer hooks (per-machine install, elevated).
; Program-based firewall rules so Windows never shows its first-run prompt
; (pressing Cancel there creates Block rules that silently win forever).
; Covers TCP and UDP (incl. mDNS on 5353) on Domain, Private and Public:
; plenty of home networks end up marked Public, and the app must work there.
; Rule names come from the product name (PRODUCTNAME, from tauri.conf.json).

!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="${PRODUCTNAME}" program="$INSTDIR\${MAINBINARYNAME}.exe"'
  nsExec::ExecToLog 'netsh advfirewall firewall add rule name="${PRODUCTNAME}" description="${PRODUCTNAME}: receive files from your other computer" dir=in action=allow program="$INSTDIR\${MAINBINARYNAME}.exe" enable=yes profile=domain,private,public protocol=TCP'
  nsExec::ExecToLog 'netsh advfirewall firewall add rule name="${PRODUCTNAME}" description="${PRODUCTNAME}: find your other computer (mDNS and broadcast)" dir=in action=allow program="$INSTDIR\${MAINBINARYNAME}.exe" enable=yes profile=domain,private,public protocol=UDP'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="${PRODUCTNAME}" program="$INSTDIR\${MAINBINARYNAME}.exe"'
!macroend
