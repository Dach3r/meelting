module Organisms
  module InboxList
    class Component < ViewComponent::Base
      def initialize(items:, preview:, assign_href:, repositories_href:)
        @items = items
        @preview = preview
        @assign_href = assign_href
        @repositories_href = repositories_href
      end

      private
      attr_reader :items, :preview, :assign_href, :repositories_href

      def subtitle
        t("home.inbox.subtitle", count: items.size, repositories: items.map(&:repository).uniq.join(", "))
      end

      def item_href(item)
        item.target == :pull_request ? repositories_href : runs_path
      end
    end
  end
end
