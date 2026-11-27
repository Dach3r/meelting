module Molecules
  module InboxRow
    class Component < ViewComponent::Base
      KINDS = {
        approve: { tone: :wait, icon: :check, button: :primary },
        merge: { tone: :neutral, icon: :git_merge, button: :graphite_outline },
        failed: { tone: :bad, icon: :triangle_alert, button: :graphite_outline }
      }.freeze

      def initialize(item:, href:)
        @item = item
        @href = href
      end

      private
      attr_reader :item, :href

      def kind
        KINDS.fetch(item.kind)
      end

      def context
        t("home.inbox.context", repository: item.repository, agent: item.agent, time: waiting)
      end

      def waiting
        minutes = item.waiting_seconds / 60
        return t("home.inbox.waiting.minutes", count: [minutes, 1].max) if minutes < 60
        return t("home.inbox.waiting.hours", count: minutes / 60) if minutes < 1440

        t("home.inbox.waiting.days", count: minutes / 1440)
      end
    end
  end
end
