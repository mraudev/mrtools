; Removes the Explorer context menu entries of mrfilesys (see src/context_menu.rs).
!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegKey HKCU "Software\Classes\Directory\shell\mrfilesys"
  DeleteRegKey HKCU "Software\Classes\Drive\shell\mrfilesys"
  DeleteRegKey HKCU "Software\Classes\Directory\Background\shell\mrfilesys"
!macroend
