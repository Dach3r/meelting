module Organisms
  module InboxList
    class Component < ViewComponent::Base
      def initialize(inbox:)
        @inbox = inbox
      end

      private
      attr_reader :inbox
    end
  end
end
