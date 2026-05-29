## Rental flow
bot-rental-request-start = Please describe your purpose for renting. You have 10 minutes.
bot-rental-answer-prefix = Answer
bot-rental-assigned = Room assigned! You now have access to { $channel }.
bot-rental-timeout = Your rental request has timed out. Please try again.
bot-rental-report = User { $user } joined a voice channel but did not submit a purpose within 10 minutes.
bot-rental-released = Room released. Thank you for using otachidai.
bot-rental-no-rooms = No rooms are currently available. Please try again later.
bot-rental-already-renting = You already have an active rental.
bot-rental-room-occupied = That room is currently occupied.
bot-rental-expired = Your rental request has expired. Please start a new request.
bot-rental-vc-question = Select the voice channel to rent
bot-rental-vc-room-occupied = The selected room is already in use. Please pick another one.
bot-rental-vc-no-rooms = No voice channels are available to choose.

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
admin-room-list-channel-registered = Room list channel registered. Room list will be posted in { $channel }.
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
    `/upload_guild_config` — Upload the whole-guild YAML configuration (channels / question presets / rooms / routing)
    `/download_guild_config` — Download the current whole-guild configuration as a YAML file
    `/list_question_presets` — List registered question presets
    `/list_rooms` — List registered rooms

## Rent button
rent-button-label = Request Room
rent-answer-button-label = Answer

## Errors
error-generic = An error occurred. Please try again.
error-db = A database error occurred. Please contact the bot operator.

## Guild configuration (YAML)
bot-config-upload-success = Guild configuration updated.
bot-config-upload-active-sessions-released = Guild configuration updated. { $count } active rental session(s) were force-released.
bot-config-upload-error-yaml = Failed to parse YAML:
    { $detail }
bot-config-upload-error-validation = Guild configuration has errors:
    { $detail }
bot-config-upload-error-attachment = Could not retrieve the uploaded YAML file. Check the size and content.
bot-config-download-empty = No guild configuration is currently registered.

## Auto-post for rental purpose
bot-rental-post-default-template = { $user } started using { $room }
    { $answers }
bot-rental-force-released = Your rental was force-released because the guild configuration was reloaded. Please request again with `/rent`.
