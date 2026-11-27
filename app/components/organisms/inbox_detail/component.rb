module Organisms
  module InboxDetail
    class Component < ViewComponent::Base
      STEP_ICONS = { command: :terminal, message: :message_square, edit: :pencil, read: :eye }.freeze

      def initialize(item:)
        @item = item
      end

      private
      attr_reader :item

      STATE_TONES = { awaiting_approval: :wait, conflicts: :bad, ready_to_merge: :ok }.freeze

      def state_tone
        STATE_TONES.fetch(item.state)
      end

      def reference
        if item.run?
          t("home.detail.run_reference", number: item.number, issue: item.detail(:issue_number))
        else
          t("home.detail.pull_request_reference", number: item.number, run: item.detail(:run_number))
        end
      end

      def commands_summary
        key = item.detail(:retried) ? "commands_retried" : "commands"
        t("home.detail.#{key}", count: item.detail(:commands))
      end
    end
  end
end
