module Organisms
  module InboxContext
    class Component < ViewComponent::Base
      MIRROR_TONES = { in_sync: :ok, failed: :bad }.freeze
      MIRROR_ICONS = { in_sync: :check, failed: :triangle_alert }.freeze

      def initialize(inbox:)
        @inbox = inbox
      end

      private
      attr_reader :inbox
    end
  end
end
