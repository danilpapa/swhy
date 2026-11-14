#!/usr/bin/env bash
# fx-rename: one symbol history with the cases listed for E4 in docs/TZ.md
# (rename, label change, move into an extension in another file, overloads,
# merge commit). Expected events are defined by the E4 tests.
#
#   1  add CardViewModel with refresh(), load(id:), load(id:completion:)
#   2  rename refresh() -> reloadCards(), body unchanged
#   3  change labels: load(id:) -> load(for:)
#   4  (branch extract-reload) move reloadCards() into an extension in
#      Cards/CardViewModel+Reload.swift
#   5  (main) change the body of load(id:completion:)
#   6  merge extract-reload into main
#   7  change the body of reloadCards() in the extension
. "$(dirname "$0")/lib.sh"

fx_init "$1"

fx_write Cards/CardViewModel.swift <<'EOF'
import Foundation

final class CardViewModel {
    let api: API
    private(set) var cards: [Card] = []

    init(api: API) {
        self.api = api
    }

    func load(id: String) {
        guard !id.isEmpty else { return }
        cards = api.cards(for: id)
    }

    func load(id: String, completion: @escaping ([Card]) -> Void) {
        guard !id.isEmpty else {
            completion([])
            return
        }
        api.fetchCards(id: id) { result in
            completion(result)
        }
    }

    func refresh() {
        let ids = cards.map { $0.id }
        cards = ids.flatMap { api.cards(for: $0) }
        notifyObservers()
    }

    private func notifyObservers() {
        NotificationCenter.default.post(name: .cardsDidChange, object: self)
    }
}
EOF
fx_commit "Add CardViewModel" "2024-01-10T10:00:00+00:00"

fx_write Cards/CardViewModel.swift <<'EOF'
import Foundation

final class CardViewModel {
    let api: API
    private(set) var cards: [Card] = []

    init(api: API) {
        self.api = api
    }

    func load(id: String) {
        guard !id.isEmpty else { return }
        cards = api.cards(for: id)
    }

    func load(id: String, completion: @escaping ([Card]) -> Void) {
        guard !id.isEmpty else {
            completion([])
            return
        }
        api.fetchCards(id: id) { result in
            completion(result)
        }
    }

    func reloadCards() {
        let ids = cards.map { $0.id }
        cards = ids.flatMap { api.cards(for: $0) }
        notifyObservers()
    }

    private func notifyObservers() {
        NotificationCenter.default.post(name: .cardsDidChange, object: self)
    }
}
EOF
fx_commit "Rename refresh to reloadCards" "2024-01-17T10:00:00+00:00"

fx_write Cards/CardViewModel.swift <<'EOF'
import Foundation

final class CardViewModel {
    let api: API
    private(set) var cards: [Card] = []

    init(api: API) {
        self.api = api
    }

    func load(for id: String) {
        guard !id.isEmpty else { return }
        cards = api.cards(for: id)
    }

    func load(id: String, completion: @escaping ([Card]) -> Void) {
        guard !id.isEmpty else {
            completion([])
            return
        }
        api.fetchCards(id: id) { result in
            completion(result)
        }
    }

    func reloadCards() {
        let ids = cards.map { $0.id }
        cards = ids.flatMap { api.cards(for: $0) }
        notifyObservers()
    }

    private func notifyObservers() {
        NotificationCenter.default.post(name: .cardsDidChange, object: self)
    }
}
EOF
fx_commit "Use load(for:) label" "2024-01-24T10:00:00+00:00"

fx_branch extract-reload
fx_write Cards/CardViewModel.swift <<'EOF'
import Foundation

final class CardViewModel {
    let api: API
    private(set) var cards: [Card] = []

    init(api: API) {
        self.api = api
    }

    func load(for id: String) {
        guard !id.isEmpty else { return }
        cards = api.cards(for: id)
    }

    func load(id: String, completion: @escaping ([Card]) -> Void) {
        guard !id.isEmpty else {
            completion([])
            return
        }
        api.fetchCards(id: id) { result in
            completion(result)
        }
    }

    func notifyObservers() {
        NotificationCenter.default.post(name: .cardsDidChange, object: self)
    }
}
EOF
fx_write Cards/CardViewModel+Reload.swift <<'EOF'
import Foundation

extension CardViewModel {
    func reloadCards() {
        let ids = cards.map { $0.id }
        cards = ids.flatMap { api.cards(for: $0) }
        notifyObservers()
    }
}
EOF
fx_commit "Move reloadCards into an extension" "2024-02-01T10:00:00+00:00"

fx_checkout main
fx_write Cards/CardViewModel.swift <<'EOF'
import Foundation

final class CardViewModel {
    let api: API
    private(set) var cards: [Card] = []

    init(api: API) {
        self.api = api
    }

    func load(for id: String) {
        guard !id.isEmpty else { return }
        cards = api.cards(for: id)
    }

    func load(id: String, completion: @escaping ([Card]) -> Void) {
        guard !id.isEmpty else {
            completion([])
            return
        }
        api.fetchCards(id: id) { [weak self] result in
            self?.cards = result
            completion(result)
        }
    }

    func reloadCards() {
        let ids = cards.map { $0.id }
        cards = ids.flatMap { api.cards(for: $0) }
        notifyObservers()
    }

    private func notifyObservers() {
        NotificationCenter.default.post(name: .cardsDidChange, object: self)
    }
}
EOF
fx_commit "Store fetched cards before calling completion" "2024-02-05T10:00:00+00:00"

fx_merge extract-reload "Merge branch 'extract-reload'" "2024-02-08T10:00:00+00:00"

fx_write Cards/CardViewModel+Reload.swift <<'EOF'
import Foundation

extension CardViewModel {
    func reloadCards() {
        let ids = Set(cards.map { $0.id })
        cards = ids.sorted().flatMap { api.cards(for: $0) }
        notifyObservers()
    }
}
EOF
fx_commit "Fix duplicated cards after reload" "2024-02-15T10:00:00+00:00"
