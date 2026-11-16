# Второй разметчик

ТЗ, E1: «Второй разметчик проверяет 30% методов; посчитать каппу Коэна». 30% от 50 — 15 методов. Они взяты первыми по номеру в каждом слое и в каждой части разбиения:

| Слой | `train` | `test` |
| --- | --- | --- |
| ObjC-происхождение (01–12) | 01, 03 | 02, 04 |
| Переезд между модулями (13–24) | 13, 15 | 14, 16 |
| Остальные (25–50) | 25, 27, 29, 31 | 26, 28, 30 |

Порядок работы:

1. Не открывать `eval/oracle/labels/signal-ios.tsv` и `eval/oracle/methods/`: там метки первого разметчика.
2. Для каждого из 15 методов взять путь и ключ из таблицы ниже и получить лист проверки: `eval/oracle/sheet.sh <путь> '<ключ>'`.
3. По правилам `docs/oracle-guide.md` записать метки в `eval/oracle/labels/signal-ios-second.tsv` — тот же формат, те же идентификаторы.
4. `SWHY_ORACLE_SECOND_ANNOTATOR=<имя> eval/oracle/build.sh` соберёт `eval/oracle/second/signal-ios-<n>.json`.
5. `eval/oracle/check.sh` посчитает каппу Коэна и запишет её в `eval/results/E1-<дата>.json`.

Состояние на 2026-11-16: разметки второго разметчика нет, каппа не посчитана.

| Метод | Путь | Ключ |
| --- | --- | --- |
| `signal-ios-01` | `SignalUI/AV/AudioPlayer.swift` | `swift:AudioPlayer.applicationDidEnterBackground()` |
| `signal-ios-02` | `SignalUI/ViewControllers/OWSNavigationController.swift` | `swift:OWSNavigationController.gestureRecognizerShouldBegin(_:)` |
| `signal-ios-03` | `SignalUI/UIKitExtensions/UIViewController+SignalUI.swift` | `swift:UIViewController.findFrontmostViewController(ignoringAlerts:)` |
| `signal-ios-04` | `Signal/ConversationView/ConversationInputToolbar.swift` | `swift:ConversationInputToolbar.viewDidAppear()` |
| `signal-ios-13` | `SignalUI/Payments/PaymentsImpl.swift` | `swift:PaymentsImpl.updateCurrentPaymentBalanceIfNecessary()` |
| `signal-ios-14` | `SignalServiceKit/Util/UIView+OWS.swift` | `swift:CGPoint.clamp(_:)` |
| `signal-ios-15` | `SignalUI/Stickers/StickerHorizontalListView.swift` | `swift:LinearHorizontalLayout.shouldInvalidateLayout(forBoundsChange:)` |
| `signal-ios-16` | `SignalServiceKit/Subscriptions/Donations/DonationSubscriptionManager.swift` | `swift:DonationSubscriptionManager.getBoostBadge()` |
| `signal-ios-25` | `Signal/src/ViewControllers/RecipientPicker/AddToBlockListViewController.swift` | `swift:AddToBlockListViewController.viewDidLoad()` |
| `signal-ios-26` | `SignalServiceKit/Mocks/Usernames/MockUsernamesService.swift` | `swift:MockUsernamesService.setUsernameLink(usernameCiphertext:keepLinkHandle:)` |
| `signal-ios-27` | `Signal/src/ViewControllers/HomeView/Stories/HiddenStoryHeaderCell.swift` | `swift:HiddenStoryHeaderCell.configure(isCollapsed:animated:)` |
| `signal-ios-28` | `SignalServiceKit/Backups/Attachments/BackupAttachmentDownloadQueueStatusManager.swift` | `swift:BackupAttachmentDownloadQueueStatusManagerImpl.downloadDidExperienceNetworkOr5xxError(mode:token:)` |
| `signal-ios-29` | `SignalServiceKit/Storage/Database/SDSCodableModel/SDSCodableModel.swift` | `swift:SDSCodableModel.didInsert(with:for:)` |
| `signal-ios-30` | `SignalServiceKit/Network/API/Requests/AuthedAccount.swift` | `swift:AuthedAccount.isAddressForLocalUser(_:)` |
| `signal-ios-31` | `Signal/DeviceTransfer/MultiPeerConnectivity/MPCDeviceTransferBrowser.swift` | `swift:MPCDeviceTransferBrowser.connectionError(_:)` |
