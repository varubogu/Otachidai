## Rental flow
bot-rental-request-start = Please describe your purpose for renting. You have 10 minutes.
bot-rental-purpose-label = Describe your purpose
bot-rental-answers-label = Answers to questions
bot-rental-answer-prefix = Answer
bot-rental-assigned = Room assigned! You now have access to { $channel }.
bot-rental-timeout = Your rental request has timed out. Please try again.
bot-rental-report = User { $user } joined a voice channel but did not submit a purpose within 10 minutes.
bot-rental-released = Room released. Thank you for using otachidai.
bot-rental-no-rooms = No rooms are currently available. Please try again later.
bot-rental-already-renting = You already have an active rental.
bot-rental-room-occupied = That room is currently occupied.
bot-rental-dropdown-prompt = Please answer the following questions, then click Confirm.
bot-rental-dropdown-confirm = Confirm
bot-rental-expired = Your rental request has expired. Please start a new request.

## Handoff
bot-handoff-prompt = The room host has left. Would you like to take over?
bot-handoff-accepted = { $user } is now the new room host.
bot-handoff-timeout = No one took over. The room has been released.
bot-handoff-take-over = Take Over

## Rental status board
status-title = Rental Status
status-available = Available
status-awaiting = Awaiting purpose
status-in-use = In use
status-pending-handoff = Awaiting handoff
status-summary = Available { $free } / In use { $used }
status-no-rooms = No rooms are registered yet.

## Admin commands
admin-report-channel-registered = Report channel registered: { $channel }
admin-rental-button-registered = Rental button channel registered. Button posted in { $channel }.
admin-question-preset-saved = Question preset saved.
admin-question-preset-name-required = Please specify a preset name.
admin-question-preset-at-least-one = Specify at least one of question_1 through question_10.
admin-question-preset-not-found = The specified question preset was not found.
admin-question-preset-deleted = Question preset deleted.
admin-question-preset-list-empty = No question presets are registered.
admin-question-preset-list-header = Registered question presets:
admin-room-registered = Room registered successfully.
admin-room-deleted = Room deleted successfully.
admin-room-not-found = No room found with those channel IDs.
admin-permission-denied = You must be a server administrator to use this command.
admin-room-at-least-one = At least one of text_channel_id or voice_channel_id is required.
admin-group-registered = Group "{ $name }" registered. Status board will be posted in { $channel }.
admin-group-deleted = Group "{ $name }" deleted.
admin-group-not-found = Group "{ $name }" not found.
admin-group-exists = Group "{ $name }" already exists.
admin-group-name-required = Please specify a group name.
admin-room-group-updated = Room moved to group "{ $name }".
admin-room-group-cleared = Room removed from its group.
admin-room-preset-updated = Room question preset changed to "{ $name }".
admin-room-preset-cleared = Room question preset cleared.
admin-room-list-empty = No rooms are registered.
admin-room-list-header = Registered rooms:
admin-room-list-item = [{ $id }] { $channels } | preset: { $preset } | group: { $group }
admin-room-list-none = (none)

## Help
help-title = otachidai Bot — Help
help-user = **User Commands**
    `/rent` — Start a rental request
    `/help` — Show this help
help-admin = **Admin Commands**
    `/register_report_channel` — Register the channel for timeout notifications
    `/register_rental_button_channel` — Register the channel where the rental button is posted
    `/register_question_preset` — Register or update a question preset for rooms
    `/list_question_presets` — List registered question presets
    `/delete_question_preset` — Delete a question preset
    `/register_room` — Register a room (text channel, voice channel, or both)
    `/list_rooms` — List registered rooms
    `/delete_room` — Delete a registered room
    `/set_room_preset` — Change a room's question preset
    `/register_group` — Register a room group (bundles rooms into one status board)
    `/delete_group` — Delete a room group
    `/set_room_group` — Change which group a room belongs to

## Rent button
rent-button-label = Request Room

## Errors
error-generic = An error occurred. Please try again.
error-db = A database error occurred. Please contact the bot operator.
